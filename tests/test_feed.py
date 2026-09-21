import copy
import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from observatory.core import Observatory, collect
from observatory.feed import MAX_BYTES, Publisher, atomic_receive, project_work, read_feed, validate_feed
from test_core import HOST, RAW


def fixture():
    app = Observatory({'hosts': [HOST]}, 'personal', lambda h: copy.deepcopy(RAW))
    app.poll(HOST)
    return app, project_work(app.snapshot(), HOST['id'])


class FeedTests(unittest.TestCase):
    def test_filter_before_transmission(self):
        app, feed = fixture()
        self.assertEqual(len(app.snapshot()['hosts'][0]['agents']), 2)
        self.assertEqual(len(feed['agents']), 1)
        for secret in ('PRIVATE', '/work/app', 'SECRET SESSION', 'history', 'cwd'):
            self.assertNotIn(secret, json.dumps(feed))

    def test_atomic_private_file_and_stale_theme(self):
        _app, feed = fixture()
        feed['theme'] = {'name': 'Source', 'colours': {'accent': '#123456'}}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'state/source.json'
            atomic_receive(path, json.dumps(feed).encode())
            self.assertEqual(os.stat(path).st_mode & 0o777, 0o600)
            host = {'id': HOST['id'], 'transport': 'file', 'path': str(path)}
            raw = read_feed(host)
            self.assertEqual(len(raw['agent_views']), 1)
            feed['captured_at'] = time.time() - 31
            atomic_receive(path, json.dumps(feed).encode())
            raw = read_feed(host)
            self.assertEqual(raw['agent_views'], [])
            self.assertIsNotNone(raw['error'])
            app = Observatory({'hosts': [host]}, 'work')
            app.poll(host)
            snapshot = app.snapshot()
            self.assertFalse(snapshot['hosts'][0]['online'])
            self.assertIsNone(snapshot['hosts'][0]['metrics'])
            self.assertEqual(snapshot['theme']['colours']['accent'], '#123456')

    def test_duplicate_sample_preserves_trend(self):
        _app, feed = fixture()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'source.json'
            atomic_receive(path, json.dumps(feed).encode())
            host = {'id': HOST['id'], 'transport': 'file', 'path': str(path)}
            app = Observatory({'hosts': [host]}, 'work')
            app.poll(host); app.poll(host)
            self.assertEqual(len(app.snapshot()['hosts'][0]['trend']), 1)
            self.assertEqual(len(app.snapshot()['history']), 1)
            self.assertEqual(app.snapshot()['hosts'][0]['sampled_at'], feed['captured_at'])
            with patch('observatory.core.time.time', return_value=feed['captured_at'] + 31):
                self.assertEqual(app.snapshot()['hosts'][0]['agents'], [])

    def test_reject_personal_invalid_future_and_wrong_host(self):
        _app, feed = fixture()
        for altered in (dict(feed, profile='personal'), dict(feed, captured_at=float('nan')), dict(feed, captured_at=time.time()+100), dict(feed, agents=[dict(feed['agents'][0], category='personal')])):
            with self.assertRaises(ValueError): validate_feed(altered)
        with self.assertRaises(ValueError): validate_feed(feed, 'wrong-host')
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError): atomic_receive(Path(directory)/'feed', b' ' * (MAX_BYTES+1))
            self.assertEqual(list(Path(directory).iterdir()), [])

    def test_allowlist_strips_unexpected_fields(self):
        _app, feed = fixture()
        feed['agents'][0]['cwd'] = '/secret'
        feed['raw'] = 'PRIVATE'
        feed['metrics']['private'] = 'PRIVATE'
        text = json.dumps(validate_feed(feed))
        self.assertNotIn('PRIVATE', text)
        self.assertNotIn('/secret', text)

    @patch('observatory.feed.subprocess.run')
    def test_publisher_only_sends_work_and_quotes_paths(self, run):
        app, _feed = fixture()
        publisher = Publisher(app, {'host_id': HOST['id'], 'target': 'office', 'directory': '/app with spaces', 'path': '/state/work.json'})
        publisher.once()
        args, kwargs = run.call_args
        self.assertNotIn('PRIVATE', kwargs['input'])
        self.assertEqual(json.loads(kwargs['input'])['profile'], 'work')
        self.assertIn("cd '/app with spaces'", args[0][-1])
        self.assertNotIn('shell', kwargs)

    def test_missing_feed_does_not_hide_local_agents(self):
        other = {'id': 'laptop', 'transport': 'file', 'path': '/nonexistent/source.json'}
        app = Observatory({'hosts': [HOST, other]}, 'work', lambda h: copy.deepcopy(RAW) if h['id'] == HOST['id'] else collect(h))
        app.poll(HOST); app.poll(other)
        self.assertTrue(app.snapshot()['hosts'][0]['online'])
        self.assertFalse(app.snapshot()['hosts'][1]['online'])
