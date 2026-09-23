import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import threading
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from observatory import probe
from test_core import SNAP


class ProbeTests(unittest.TestCase):
    def test_remote_gpu_does_not_follow_dashboard_redirects(self):
        hits = []
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                hits.append(self.path)
                if self.path == '/api/state':
                    self.send_response(302)
                    self.send_header('Location', '/private')
                else:
                    self.send_response(200)
                self.end_headers()

            def log_message(self, *_args):
                pass

        with ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                self.assertIsNone(probe.dashboard_gpu(server.server_port, 'ws-255'))
                self.assertEqual(hits, ['/api/state'])
            finally:
                server.shutdown()
                thread.join()

    def test_remote_gpu_has_wall_clock_deadline_for_slow_body(self):
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'{')
                self.wfile.flush()
                time.sleep(3)

            def log_message(self, *_args):
                pass

        with ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                started = time.monotonic()
                self.assertIsNone(probe.dashboard_gpu(server.server_port, 'ws-255'))
                self.assertLess(time.monotonic() - started, 2.8)
            finally:
                server.shutdown()
                thread.join()

    @patch('observatory.probe.urllib.request.build_opener')
    def test_remote_loopback_dashboard_gpu_is_host_bound_and_fresh(self, opener):
        response = opener.return_value.open.return_value.__enter__.return_value
        host = {'id': 'ws-255', 'online': True, 'sampled_at': 100,
                'metrics': {'gpu': {'percent': 0, 'used': 100, 'total': 200, 'source': 'nvidia-visible'}}}
        response.read.return_value = json.dumps({'hosts': [{'id': 'other', 'online': True, 'sampled_at': 100, 'metrics': {'gpu': {'percent': 99}}}, host]}).encode()
        self.assertEqual(probe.dashboard_gpu(8789, 'ws-255', now=105), host['metrics']['gpu'])
        opener.return_value.open.assert_called_with('http://127.0.0.1:8789/api/state', timeout=2)
        host['sampled_at'] = 80
        response.read.return_value = json.dumps({'hosts': [host]}).encode()
        self.assertIsNone(probe.dashboard_gpu(8789, 'ws-255', now=105))
        host['sampled_at'] = 100
        host['metrics']['gpu']['percent'] = 101
        response.read.return_value = json.dumps({'hosts': [host]}).encode()
        self.assertIsNone(probe.dashboard_gpu(8789, 'ws-255', now=105))
        response.read.return_value = b'x' * 1048577
        self.assertIsNone(probe.dashboard_gpu(8789, 'ws-255', now=105))
        response.read.return_value = ('[' * 600 + '0' + ']' * 600).encode()
        self.assertIsNone(probe.dashboard_gpu(8789, 'ws-255', now=105))
        self.assertIsNone(probe.dashboard_gpu(0, 'ws-255', now=105))

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
