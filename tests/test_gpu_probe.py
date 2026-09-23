import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from observatory import gpu_probe


class GpuProbeTests(unittest.TestCase):
    def test_busiest_engine_and_counter_reset(self):
        before = [(10, 100), (20, 100), (50, 100)]
        after = [(40, 200), (20, 200), (10, 200)]
        self.assertEqual(gpu_probe.utilisation(before, after), 30)
        self.assertEqual(gpu_probe.utilisation([(10, 100)], [(10, 200)]), 0)
        self.assertIsNone(gpu_probe.utilisation([(10, 100)], [(11, 100)]))

    def test_publication_contains_only_aggregate(self):
        with TemporaryDirectory() as directory:
            output = Path(directory) / 'metrics.json'
            gpu_probe.publish(25.5, output, now=123)
            self.assertEqual(json.loads(output.read_text()), {'at': 123, 'percent': 25.5, 'source': 'intel-xe-pmu'})
            self.assertEqual(output.stat().st_mode & 0o777, 0o644)
            with self.assertRaises(ValueError):
                gpu_probe.publish(101, output, now=124)

    def test_discovery_opens_only_valid_engine_pairs(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            device = root / 'xe_0000_00_02.0'
            device.mkdir()
            (device / 'type').write_text('16')
            for relative, value in (('events/engine-active-ticks', 'event=0x02'), ('events/engine-total-ticks', 'event=0x03'), ('format/engine_class', 'config:20-27'), ('format/engine_instance', 'config:12-19')):
                target = device / relative
                target.parent.mkdir(exist_ok=True)
                target.write_text(value)
            with patch('observatory.gpu_probe.platform.machine', return_value='x86_64'), patch('observatory.gpu_probe.open_event', side_effect=OSError) as opening:
                self.assertEqual(gpu_probe.discover(root), [])
                self.assertEqual(opening.call_count, 80)
            (device / 'format/engine_class').write_text('config:19-27')
            with patch('observatory.gpu_probe.platform.machine', return_value='x86_64'), patch('observatory.gpu_probe.open_event') as opening:
                self.assertEqual(gpu_probe.discover(root), [])
                opening.assert_not_called()
            with patch('observatory.gpu_probe.platform.machine', return_value='aarch64'):
                self.assertEqual(gpu_probe.discover(root), [])


if __name__ == '__main__':
    unittest.main()
