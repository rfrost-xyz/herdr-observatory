import json
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from observatory import probe
from test_core import SNAP


class ProbeTests(unittest.TestCase):
    def test_xe_gpu_requires_fresh_bounded_aggregate(self):
        from pathlib import Path
        from tempfile import TemporaryDirectory
        with TemporaryDirectory() as directory:
            path = Path(directory) / 'metrics.json'
            path.write_text(json.dumps({'at': 100, 'percent': 25.5, 'source': 'intel-xe-pmu'}))
            self.assertEqual(probe.xe_gpu(path, now=105), {'percent': 25.5, 'source': 'intel-xe-pmu'})
            self.assertIsNone(probe.xe_gpu(path, now=111))
            for sample in ({'at': 106, 'percent': 1, 'source': 'intel-xe-pmu'}, {'at': 100, 'percent': 101, 'source': 'intel-xe-pmu'}, {'at': 100, 'percent': 0, 'source': 'unknown'}):
                path.write_text(json.dumps(sample))
                self.assertIsNone(probe.xe_gpu(path, now=105))
            path.write_text('x' * 513)
            self.assertIsNone(probe.xe_gpu(path, now=105))
            path.write_text(json.dumps({'at': 10 ** 400, 'percent': 1, 'source': 'intel-xe-pmu'}))
            self.assertIsNone(probe.xe_gpu(path, now=105))

    @patch('observatory.probe.shutil.which', return_value='/bin/nvidia-smi')
    @patch('observatory.probe.subprocess.run')
    def test_valid_nvidia_sample_names_visible_scope(self, run, _which):
        run.return_value = SimpleNamespace(stdout='12, 100, 200\n')
        self.assertEqual(probe.metrics()['gpu'], {'percent': 12.0, 'used': 104857600.0, 'total': 209715200.0, 'source': 'nvidia-visible'})

    @patch('observatory.probe.palette', return_value=None)
    @patch('observatory.probe.metrics', return_value={'at': 1})
    @patch('observatory.probe.shutil.which', return_value='/bin/herdr')
    @patch('observatory.probe.subprocess.run')
    def test_snapshot_is_allowlisted_and_session_selected(self, run, *_mocks):
        run.return_value = SimpleNamespace(stdout=json.dumps({'result': {'snapshot': SNAP}}))
        result = probe.sample(session='work')
        self.assertEqual(run.call_args.args[0], ['/bin/herdr', '--session', 'work', 'api', 'snapshot'])
        self.assertNotIn('SECRET SESSION', json.dumps(result))
        self.assertEqual(result['snapshot']['agents'][0]['agent_status'], 'working')

    @patch('observatory.probe.palette', return_value=None)
    @patch('observatory.probe.metrics', return_value={'at': 1})
    @patch('observatory.probe.subprocess.run', side_effect=FileNotFoundError)
    def test_missing_herdr_keeps_metrics(self, *_mocks):
        result = probe.sample()
        self.assertIsNone(result['snapshot'])
        self.assertEqual(result['metrics'], {'at': 1})
        self.assertIsNotNone(result['error'])

    @patch('observatory.probe.shutil.which', return_value='/bin/nvidia-smi')
    @patch('observatory.probe.subprocess.run')
    def test_malformed_gpu_preserves_other_metrics(self, run, _which):
        for output in ('10, 100\n', '10, 100, 200, 300\n', 'unsupported\n'):
            run.return_value = SimpleNamespace(stdout=output)
            result = probe.metrics()
            self.assertIsNone(result['gpu'])
            self.assertIsNotNone(result['cpu'])
            self.assertIsNotNone(result['memory'])

    @patch('observatory.probe.palette', return_value=None)
    @patch('observatory.probe.shutil.which', return_value='/bin/tool')
    @patch('observatory.probe.subprocess.run')
    def test_malformed_gpu_does_not_hide_agents(self, run, *_mocks):
        run.side_effect = [SimpleNamespace(stdout='10, 100\n'), SimpleNamespace(stdout=json.dumps({'result': {'snapshot': SNAP}}))]
        result = probe.sample()
        self.assertIsNone(result['error'])
        self.assertEqual(result['snapshot']['agents'][0]['agent_status'], 'working')
        self.assertIsNone(result['metrics']['gpu'])
        self.assertIsNotNone(result['metrics']['cpu'])


class SocketTests(unittest.TestCase):
    @patch('observatory.probe.socket.socket')
    def test_socket_request_is_read_only_and_response_is_allowlisted(self, socket):
        client = socket.return_value.__enter__.return_value
        payload = json.dumps({'id': 'observatory-snapshot', 'result': {'snapshot': SNAP}}).encode() + b'\n'
        client.recv.side_effect = [payload[:20], payload[20:]]
        with patch('observatory.probe.metrics', return_value={'at': 1}), patch('observatory.probe.palette', return_value=None):
            result = probe.sample(socket_path='/herdr/herdr.sock')
        self.assertIsNone(result['error'])
        self.assertNotIn('SECRET SESSION', json.dumps(result))
        sent = json.loads(client.sendall.call_args.args[0])
        self.assertEqual(sent, {'id': 'observatory-snapshot', 'method': 'session.snapshot', 'params': {}})
        client.connect.assert_called_once_with('/herdr/herdr.sock')

    @patch('observatory.probe.socket.socket')
    def test_socket_failure_frames_are_rejected(self, socket):
        client = socket.return_value.__enter__.return_value
        for payload in (b'{}\n', b'invalid\n', b'[]\n', b'', b'{"id":"other","result":{}}\n', b'{"id":"observatory-snapshot","error":{}}\n'):
            client.recv.side_effect = [payload]
            with self.assertRaises((ValueError, KeyError)):
                probe.socket_snapshot('/herdr/herdr.sock')
        client.recv.side_effect = [b'x' * 65536] * 65
        with self.assertRaises(ValueError):
            probe.socket_snapshot('/herdr/herdr.sock')

    @patch('observatory.probe.socket.socket')
    @patch('observatory.probe.time.monotonic', side_effect=[0, 0, 7])
    def test_socket_deadline_rejects_slow_response(self, _clock, socket):
        socket.return_value.__enter__.return_value.recv.return_value = b'x'
        with self.assertRaises(ValueError):
            probe.socket_snapshot('/herdr/herdr.sock')
