"""Music metadata must remain bounded and independent of Herdr collection."""
import io
import json
from pathlib import Path
import socket
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from observatory.music import Music, receive, sanitise, unavailable, validate_config


class MusicTests(unittest.TestCase):
    def frame(self, **values):
        return {'available': True, 'state': 'playing', 'title': 'Title', 'artist': 'Artist', 'bands': [.1, .8], 'captured_at': 100, **values}

    def test_allowlist_and_expiry(self):
        raw = self.frame(path='/private/song', provider_meta={'secret': 'x'}, title='hello\nworld')
        result = sanitise(raw, 101)
        self.assertEqual(result['title'], 'helloworld')
        self.assertEqual(set(result), set(unavailable()))
        self.assertEqual(sanitise(raw, 104), unavailable())
        self.assertEqual(sanitise(raw, 99), unavailable())
        self.assertEqual(sanitise(self.frame(state='paused'), 100)['bands'], [0, 0])

    def test_invalid_frames_fail_closed(self):
        for change in [{'captured_at': float('nan')}, {'captured_at': 10 ** 400}, {'bands': [10 ** 400]}, {'captured_at': True}, {'bands': [float('inf')]}, {'bands': [True]}, {'bands': [-.1]}, {'bands': [1.1]}, {'bands': []}, {'bands': [0] * 65}, {'state': 'invented'}]:
            with self.subTest(change=change):
                self.assertEqual(sanitise(self.frame(**change), 100), unavailable())

    def test_config_is_explicit_and_no_command_injection(self):
        validate_config(None)
        validate_config({'socket_path': '/music/cliamp.sock', 'publish': {'target': 'user@host', 'container': 'observatory', 'path': '/feeds/music.json'}})
        for config in [{}, {'socket_path': '/a', 'path': '/b'}, {'path': '../bad'}, {'socket_path': '/x', 'publish': {'target': '-oProxyCommand=x', 'container': 'x', 'path': '/x'}}]:
            with self.assertRaises(ValueError):
                validate_config(config)

    def test_missing_socket_does_not_escape_thread_and_closes(self):
        with tempfile.TemporaryDirectory() as root:
            music = Music({'socket_path': root + '/missing'})
            music.start()
            time.sleep(.08)
            self.assertEqual(music.snapshot(), unavailable())
            music.close()
            self.assertFalse(music._thread.is_alive())

    def test_persistent_socket_requests_are_only_read_methods(self):
        with tempfile.TemporaryDirectory() as root:
            path = root + '/socket'
            server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            server.bind(path)
            server.listen()
            requests = []
            def reply():
                conn, _ = server.accept()
                with conn, conn.makefile('rb') as stream:
                    for _ in range(4):
                        request = json.loads(stream.readline())
                        requests.append(request)
                        response = {'version': 2, 'id': request['id'], 'ok': True}
                        if request['method'] == 'state.get':
                            response['snapshot'] = {'state': 'playing', 'track': {'title': 'Song', 'artist': 'Artist', 'path': '/private'}}
                        else:
                            response['result'] = {'ok': True, 'bands': [.2, .6]}
                        payload = (json.dumps(response) + '\n').encode()
                        conn.sendall(payload[:4])
                        conn.sendall(payload[4:])
            worker = threading.Thread(target=reply)
            worker.start()
            music = Music({'socket_path': path})
            try:
                for _ in range(2):
                    result = music._sample()
                    self.assertTrue(result['available'])
                    self.assertNotIn('path', result)
                self.assertEqual([r['method'] for r in requests], ['state.get', 'spectrum.get'] * 2)
                with self.assertRaises(ValueError):
                    music._request('operation.submit')
            finally:
                music.close()
                worker.join(timeout=2)
                server.close()

    def test_remote_timestamp_is_preserved_and_expires(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'music.json'
            with patch('observatory.music.time.time', return_value=101):
                receive(path, io.BytesIO((json.dumps(self.frame()) + '\n').encode()))
                music = Music({'path': str(path)})
                self.assertEqual(music._sample()['captured_at'], 100)
            with patch('observatory.music.time.time', return_value=104):
                self.assertEqual(music._sample(), unavailable())
            receive(path, io.BytesIO((json.dumps(self.frame(captured_at=10 ** 400)) + '\n').encode()))
            self.assertEqual(json.loads(path.read_text()), unavailable())
            receive(path, io.BytesIO(b'{bad}\n'))
            self.assertEqual(json.loads(path.read_text()), unavailable())
            self.assertEqual(list(Path(root).glob('.music-*')), [])

    def test_receiver_limits_frames_and_writes_only_configured_file(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'music.json'
            receive(path, io.BytesIO(b'x' * 4097 + b'\n'))
            self.assertFalse(path.exists())
            receive(path, io.BytesIO(b'{}'))
            self.assertFalse(path.exists())

    def test_publish_outage_is_bounded(self):
        config = {'socket_path': '/music/socket', 'publish': {'target': 'host', 'container': 'obs', 'path': '/feeds/music.json'}}
        music = Music(config)
        with patch('observatory.music.subprocess.Popen', side_effect=OSError('offline')) as launch:
            music._publish(unavailable())
            music._publish(unavailable())
            self.assertEqual(launch.call_count, 1)
        music.close()
