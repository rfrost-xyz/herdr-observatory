"""Isolated Intel Xe PMU sampler; publishes only aggregate graphics utilisation."""
import ctypes
import json
import math
import os
from pathlib import Path
import platform
import tempfile
import time


PMU_ROOT = Path('/sys/bus/event_source/devices')
OUTPUT = Path('/gpu/metrics.json')
PERF_EVENT_OPEN_X86_64 = 298


class PerfEventAttr(ctypes.Structure):
    _fields_ = [('type', ctypes.c_uint), ('size', ctypes.c_uint),
                ('config', ctypes.c_ulonglong), ('reserved', ctypes.c_byte * 112)]


def open_event(pmu_type, event, engine_class, instance):
    config = event | (instance << 12) | (engine_class << 20)
    attr = PerfEventAttr(pmu_type, ctypes.sizeof(PerfEventAttr), config)
    libc = ctypes.CDLL(None, use_errno=True)
    fd = libc.syscall(PERF_EVENT_OPEN_X86_64, ctypes.byref(attr), -1, 0, -1, 0)
    if fd < 0:
        raise OSError(ctypes.get_errno(), 'perf_event_open failed')
    return fd


def discover(root=PMU_ROOT):
    if platform.machine() != 'x86_64':
        return []
    events = []
    for device in sorted(root.glob('xe_*')):
        try:
            pmu_type = int((device / 'type').read_text())
            if ((device / 'events/engine-active-ticks').read_text().strip() != 'event=0x02' or
                    (device / 'events/engine-total-ticks').read_text().strip() != 'event=0x03' or
                    (device / 'format/engine_class').read_text().strip() != 'config:20-27' or
                    (device / 'format/engine_instance').read_text().strip() != 'config:12-19'):
                continue
        except (OSError, ValueError):
            continue
        for engine_class in range(5):
            for instance in range(16):
                active = total = None
                try:
                    active = open_event(pmu_type, 2, engine_class, instance)
                    total = open_event(pmu_type, 3, engine_class, instance)
                    events.append((active, total))
                except OSError:
                    if active is not None:
                        os.close(active)
                    if total is not None:
                        os.close(total)
    return events


def counters(events):
    values = []
    for active, total in events:
        busy_bytes, total_bytes = os.read(active, 8), os.read(total, 8)
        if len(busy_bytes) != 8 or len(total_bytes) != 8:
            raise ValueError('Short XE PMU counter')
        values.append((int.from_bytes(busy_bytes, 'little'), int.from_bytes(total_bytes, 'little')))
    return values


def utilisation(before, after):
    values = []
    for (busy_old, total_old), (busy_new, total_new) in zip(before, after):
        busy, total = busy_new - busy_old, total_new - total_old
        if total > 0 and 0 <= busy <= total:
            values.append(100 * busy / total)
    return round(max(values), 1) if values else None


def publish(percent, output=OUTPUT, now=None):
    now = time.time() if now is None else now
    if not math.isfinite(percent) or not 0 <= percent <= 100:
        raise ValueError('Invalid XE utilisation')
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(mode='w', dir=output.parent, prefix='.metrics-', delete=False) as stream:
        temporary = Path(stream.name)
        json.dump({'at': now, 'percent': percent, 'source': 'intel-xe-pmu'}, stream)
    os.chmod(temporary, 0o644)
    os.replace(temporary, output)


def main():
    events = discover()
    try:
        while True:
            try:
                before = counters(events)
                time.sleep(2)
                percent = utilisation(before, counters(events))
                if percent is not None:
                    publish(percent)
                else:
                    OUTPUT.unlink(missing_ok=True)
            except (OSError, ValueError):
                OUTPUT.unlink(missing_ok=True)
                time.sleep(2)
    finally:
        for active, total in events:
            os.close(active)
            os.close(total)


if __name__ == '__main__':
    main()
