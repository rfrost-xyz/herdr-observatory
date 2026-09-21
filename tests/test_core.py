import copy
import json
import subprocess
import unittest
from unittest.mock import patch

from observatory.core import Observatory, classification, collect, normalise, rates, theme, validate_config

HOST = {'id': 'desktop', 'work_roots': ['/work'], 'personal_roots': ['/work/private']}
SNAP = {'version': '0.9.0', 'workspaces': [{'workspace_id': 'w1', 'label': 'Public project'}, {'workspace_id': 'w2', 'label': 'PRIVATE PROJECT'}], 'agents': [
    {'pane_id': 'p1', 'workspace_id': 'w1', 'cwd': '/work/app', 'agent': 'codex', 'agent_status': 'working', 'terminal_title_stripped': 'Run checks', 'agent_session': {'value': 'SECRET SESSION'}},
    {'pane_id': 'p2', 'workspace_id': 'w2', 'cwd': '/personal/secret', 'agent': 'codex', 'agent_status': 'blocked', 'terminal_title_stripped': 'PRIVATE TASK'}]}
RAW = {'snapshot': SNAP, 'error': None, 'theme': None, 'metrics': {'at': 10, 'scope': 'Linux', 'cpu': {'total': 100, 'idle': 50}, 'network': {'rx': 100, 'tx': 200}}}


class CoreTests(unittest.TestCase):
    def test_work_disclosure_is_server_side(self):
        app = Observatory({'hosts': [HOST]}, 'work', lambda h: copy.deepcopy(RAW))
        app.poll(HOST)
        payload = json.dumps(app.snapshot())
        for secret in ('PRIVATE', 'SECRET SESSION', '/work/app', '/personal/secret', 'agent_session', 'cwd'):
            self.assertNotIn(secret, payload)
        self.assertEqual(len(app.snapshot()['hosts'][0]['agents']), 1)
        self.assertEqual(len(app.snapshot()['history']), 1)

    def test_technical_metadata_is_typed_and_private_agents_stay_excluded(self):
        sample = copy.deepcopy(SNAP)
        sample['agents'][0].update(revision=32, state_change_seq=7, focused=True,
                                   interactive_ready='yes', launch_pending=False,
                                   tokens={'secret': 'PRIVATE TOKEN'}, state_labels={'secret': 'PRIVATE LABEL'})
        sample['agents'][1]['revision'] = 987654321
        agents = normalise(sample, HOST, 'work')
        self.assertEqual(agents[0]['technical'], {'revision':32, 'state_change_seq':7,
            'focused':True, 'interactive_ready':None, 'launch_pending':False})
        self.assertNotIn('PRIVATE', json.dumps(agents))
        self.assertNotIn('987654321', json.dumps(agents))
        for invalid in (True, -1, 1.5, '12', 2**64):
            sample['agents'][0]['revision'] = invalid
            self.assertIsNone(normalise(sample, HOST, 'work')[0]['technical']['revision'])

    def test_personal_profile_includes_both_categories(self):
        agents = normalise(SNAP, HOST, 'personal')
        self.assertEqual([a['category'] for a in agents], ['work', 'personal'])

    def test_path_boundaries_and_personal_precedence(self):
        for path in ('/work/private/x', '/workshop/x', '/work/../private', None, 'relative'):
            self.assertEqual(classification(path, HOST), 'personal')
        self.assertEqual(classification('/work/a', HOST), 'work')
        self.assertEqual(classification('/work', HOST), 'work')

    def test_failure_clears_working_and_recovery_reseeds(self):
        app = Observatory({'hosts': [HOST]}, 'personal', lambda h: copy.deepcopy(RAW))
        app.poll(HOST)
        app.collector = lambda h: (_ for _ in ()).throw(subprocess.TimeoutExpired('ssh', 15))
        app.poll(HOST)
        state = app.snapshot()['hosts'][0]
        self.assertFalse(state['online'])
        self.assertEqual(state['agents'], [])
        self.assertIsNone(state['metrics'])
        app.collector = lambda h: copy.deepcopy(RAW)
        app.poll(HOST)
        self.assertTrue(app.snapshot()['hosts'][0]['online'])
        self.assertIsNone(app.snapshot()['hosts'][0]['metrics']['cpu_percent'])

    def test_missing_snapshot_marks_herdr_unavailable(self):
        raw = copy.deepcopy(RAW); raw['snapshot'] = None
        app = Observatory({'hosts': [HOST]}, collector=lambda h: raw)
        app.poll(HOST)
        self.assertFalse(app.snapshot()['hosts'][0]['online'])

    def test_history_bounded_and_unchanged_samples_not_events(self):
        raw = copy.deepcopy(RAW)
        app = Observatory({'hosts': [HOST]}, collector=lambda h: raw)
        app.poll(HOST); app.poll(HOST)
        self.assertEqual(len(app.history), 1)
        for i in range(130):
            raw['snapshot']['agents'][0]['agent_status'] = 'idle' if i % 2 else 'working'
            app.poll(HOST)
        self.assertEqual(len(app.history), 100)
        self.assertEqual(len(app.snapshot()['hosts'][0]['trend']), 60)

    def test_host_identity_prevents_pane_collisions(self):
        other = dict(HOST, id='second')
        app = Observatory({'hosts': [HOST, other]}, collector=lambda h: copy.deepcopy(RAW))
        app.poll(HOST); app.poll(other)
        self.assertEqual(len({a['id'] for h in app.snapshot()['hosts'] for a in h['agents']}), 2)

    def test_theme_validates_values_and_falls_back(self):
        t = theme({'name': 'Light', 'colours': {'accent': '#ABCDEF', 'background': 'url(evil)', 'injected': '#ffffff'}})
        self.assertEqual(t['colours']['accent'], '#ABCDEF')
        self.assertEqual(t['colours']['background'], '#1a1b26')
        self.assertNotIn('injected', t['colours'])
        self.assertEqual(theme(None)['colours']['accent'], '#7aa2f7')

    def test_theme_updates_between_samples(self):
        raw = copy.deepcopy(RAW)
        app = Observatory({'hosts': [HOST]}, collector=lambda h: raw)
        app.poll(HOST)
        raw['theme'] = {'name': 'New', 'colours': {'accent': '#123456'}}
        app.poll(HOST)
        self.assertEqual(app.snapshot()['theme']['colours']['accent'], '#123456')

    def test_rates_and_counter_reset(self):
        previous = RAW['metrics']
        current = {'at': 12, 'cpu': {'total': 200, 'idle': 70}, 'network': {'rx': 300, 'tx': 400}}
        self.assertEqual(rates(current, previous)['cpu_percent'], 80)
        self.assertEqual(rates(current, previous)['rx_rate'], 100)
        self.assertIsNone(rates(previous, current)['rx_rate'])
        self.assertIsNone(rates(previous, None)['cpu_percent'])

    def test_invalid_config_and_snapshot(self):
        for config in ({'hosts': []}, {'hosts': [HOST, HOST]}, {'hosts': [dict(HOST, transport='ssh', target='-oProxyCommand=evil')]}, {'hosts': [dict(HOST, work_roots=['relative'])]}, {'hosts': [HOST], 'interval': 0}):
            with self.assertRaises(ValueError): validate_config(config)
        with self.assertRaises(ValueError): normalise({'agents': None}, HOST, 'work')

    @patch('observatory.core.subprocess.run')
    def test_ssh_uses_fixed_command_and_stdin(self, run):
        run.return_value.stdout = json.dumps(RAW)
        collect(dict(HOST, transport='ssh', target='my-alias'))
        args, kwargs = run.call_args
        self.assertEqual(args[0][-3:], ['--', 'my-alias', 'python3 -'])
        self.assertIn("['api', 'snapshot']", kwargs['input'])
        self.assertNotIn('shell', kwargs)
        self.assertEqual(kwargs['timeout'], 15)

    def test_snapshot_copy_cannot_mutate_state(self):
        app = Observatory({'hosts': [HOST]}, collector=lambda h: copy.deepcopy(RAW)); app.poll(HOST)
        app.snapshot()['hosts'][0]['agents'].clear()
        self.assertEqual(len(app.snapshot()['hosts'][0]['agents']), 1)

    def test_malformed_metrics_and_extra_fields_do_not_escape(self):
        raw = copy.deepcopy(RAW)
        raw['metrics']['private'] = 'SECRET'
        raw['metrics']['gpu'] = {'percent': float('nan'), 'used': 1, 'total': 2}
        app = Observatory({'hosts': [HOST]}, collector=lambda h: raw)
        app.poll(HOST)
        self.assertNotIn('SECRET', json.dumps(app.snapshot(), allow_nan=False))
        self.assertIsNone(app.snapshot()['hosts'][0]['metrics']['gpu'])
        raw['metrics'] = []
        app.poll(HOST)
        self.assertFalse(app.snapshot()['hosts'][0]['online'])
        raw['metrics'] = copy.deepcopy(RAW['metrics'])
        app.poll(HOST)
        self.assertTrue(app.snapshot()['hosts'][0]['online'])

    def test_unavailable_herdr_and_transport_leave_unknown_trend(self):
        raw = copy.deepcopy(RAW)
        app = Observatory({'hosts': [HOST]}, collector=lambda h: raw)
        app.poll(HOST)
        raw['snapshot'] = None
        app.poll(HOST)
        self.assertIsNone(app.snapshot()['hosts'][0]['trend'][-1]['working'])
        app.collector = lambda h: (_ for _ in ()).throw(OSError())
        app.poll(HOST)
        self.assertIsNone(app.snapshot()['hosts'][0]['trend'][-1]['working'])
        app.collector = lambda h: copy.deepcopy(RAW)
        app.poll(HOST)
        self.assertEqual(app.snapshot()['hosts'][0]['trend'][-1]['working'], 1)
