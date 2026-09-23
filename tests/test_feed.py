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

    def test_numeric_session_generation_survives_work_validation(self):
        app, feed = fixture()
        feed['agents'][0]['technical']['session_generation'] = 123
        feed['agents'][0]['technical']['session_id'] = 'PRIVATE'
        validated = validate_feed(feed)
        self.assertEqual(validated['agents'][0]['technical']['session_generation'], 123)
        self.assertNotIn('PRIVATE', json.dumps(validated))

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
            path.write_text(json.dumps(feed))
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

    @patch('observatory.feed.subprocess.run')
    def test_publisher_uses_receiver_image_without_remote_checkout(self, run):
        app, _feed = fixture()
        Publisher(app, {'host_id': HOST['id'], 'target': 'office', 'container': 'herdr-observatory', 'path': '/feeds/work.json'}).once()
        self.assertEqual(run.call_args.args[0][-1], 'docker exec -i herdr-observatory python3 -m observatory.feed /feeds/work.json')
        self.assertNotIn('PRIVATE', run.call_args.kwargs['input'])

    def test_feed_preserves_only_typed_technical_metadata(self):
        _app, feed = fixture()
        feed['protocol'] = 22
        feed['agents'][0]['technical'] = {'revision':12, 'state_change_seq':8, 'focused':False,
            'interactive_ready':True, 'launch_pending':False, 'tokens':'PRIVATE'}
        value = validate_feed(feed)
        self.assertEqual(value['protocol'],22)
        self.assertEqual(value['agents'][0]['technical']['revision'],12)
        self.assertNotIn('PRIVATE',json.dumps(value))
        feed['protocol'] = True
        feed['agents'][0]['technical'] = {'revision':True,'focused':'yes'}
        value=validate_feed(feed)
        self.assertIsNone(value['protocol'])
        self.assertIsNone(value['agents'][0]['technical']['revision'])
        self.assertIsNone(value['agents'][0]['technical']['focused'])

    def test_missing_feed_does_not_hide_local_agents(self):
        other = {'id': 'laptop', 'transport': 'file', 'path': '/nonexistent/source.json'}
        app = Observatory({'hosts': [HOST, other]}, 'work', lambda h: copy.deepcopy(RAW) if h['id'] == HOST['id'] else collect(h))
        app.poll(HOST); app.poll(other)
        self.assertTrue(app.snapshot()['hosts'][0]['online'])
        self.assertFalse(app.snapshot()['hosts'][1]['online'])

    def test_older_feed_cannot_resurrect_state_or_theme_across_restart(self):
        _app, newer = fixture()
        newer['captured_at'] = time.time() - 2
        newer['agents'][0]['status'] = 'done'
        newer['theme'] = {'name': 'New', 'colours': {'accent': '#123456'}}
        older = copy.deepcopy(newer)
        older['captured_at'] -= 8
        older['agents'][0]['status'] = 'working'
        older['theme']['name'] = 'Old'
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'feed.json'
            self.assertTrue(atomic_receive(path, json.dumps(newer).encode()))
            self.assertFalse(atomic_receive(path, json.dumps(older).encode()))
            host = {'id': HOST['id'], 'transport': 'file', 'path': str(path)}
            app = Observatory({'hosts': [host]}, 'work')
            app.poll(host)
            self.assertEqual(app.snapshot()['hosts'][0]['agents'][0]['status'], 'done')
            self.assertEqual(app.snapshot()['theme']['name'], 'New')
            # Defence in depth even if a file was replaced outside the receiver.
            path.write_text(json.dumps(older))
            app.poll(host)
            self.assertEqual(app.snapshot()['hosts'][0]['agents'][0]['status'], 'done')
            self.assertEqual(app.snapshot()['theme']['name'], 'New')
            self.assertEqual(len(app.snapshot()['history']), 1)

    def test_equal_capture_only_allows_offline_transition(self):
        _app, feed = fixture()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'feed.json'
            atomic_receive(path, json.dumps(feed).encode())
            changed = copy.deepcopy(feed)
            changed['agents'][0]['status'] = 'done'
            self.assertFalse(atomic_receive(path, json.dumps(changed).encode()))
            self.assertTrue(atomic_receive(path, json.dumps(dict(feed, online=False)).encode()))
            self.assertFalse(atomic_receive(path, json.dumps(feed).encode()))
            self.assertFalse(json.loads(path.read_text())['online'])
