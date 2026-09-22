import copy
import importlib.util
import json
from pathlib import Path
import socket
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from observatory.core import Observatory
from observatory.feed import project_work
from observatory.probe import session_binding, telemetry_from_agent, telemetry_view
from observatory.telemetry import event_view, report, rpc
from test_core import HOST, RAW

import observatory.telemetry
INSTALL = Path(observatory.telemetry.__file__).resolve().parents[1] / 'hooks/install.py'
spec = importlib.util.spec_from_file_location('hook_install', INSTALL)
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


def agent(harness='codex'):
    return {'agent': harness, 'pane_id': 'w1:p1', 'agent_status': 'working',
            'agent_session': {'agent': harness, 'source': 'herdr:' + harness, 'kind': 'id', 'value': 'native-secret'}}


def event():
    return {'seq': int(time.time() * 1e6), 'event': 'tool-start', 'phase': 'tool', 'tool': 'bash', 'model': 'test-model'}


class TelemetryTests(unittest.TestCase):
    def test_report_is_presentation_only_bound_and_clears_absent_fields(self):
        a = agent()
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / 'config.json'
            config.write_text(json.dumps({'hosts': [{'socket_path': '/herdr/herdr.sock'}]}))
            with patch('observatory.telemetry.rpc', side_effect=[{'pane': a}, {}]) as call:
                self.assertTrue(report('codex', {'session_id': 'native-secret', 'hook_event_name': 'PreToolUse', 'tool_name': 'Bash', 'tool_input': 'SECRET', 'model': 'test-model'}, 'w1:p1', event()['seq'], config))
                self.assertEqual([c.args[1] for c in call.call_args_list], ['pane.get', 'pane.report_metadata'])
                params = call.call_args.args[2]
                self.assertEqual(params['tokens']['obs_bind'], session_binding(a))
                self.assertIsNone(params['tokens']['obs_input'])
                self.assertLessEqual(len(params['tokens']), 16)
                self.assertEqual(params['ttl_ms'], 120000)
                self.assertNotIn('SECRET', json.dumps(params))
                a['tokens'] = params['tokens']
                decoded = telemetry_from_agent(a)
                self.assertEqual(decoded['tool'], 'Bash')
                self.assertNotIn('native-secret', json.dumps(decoded))
                a['agent_session']['value'] = 'replacement'
                self.assertIsNone(telemetry_from_agent(a))

    def test_replaced_out_of_order_and_wrong_harness_reports_are_dropped(self):
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / 'config.json'
            config.write_text(json.dumps({'hosts': [{'socket_path': '/herdr/herdr.sock'}]}))
            for kind in ('session', 'order', 'harness', 'source'):
                a = agent();seq = event()['seq']
                if kind == 'session': a['agent_session']['value'] = 'replacement'
                if kind == 'order': a['tokens'] = {'obs_seq': str(seq + 1)}
                if kind == 'harness': a['agent'] = 'pi'
                if kind == 'source': a['agent_session']['source'] = 'user:other'
                with patch('observatory.telemetry.rpc', return_value={'pane': a}) as call:
                    self.assertFalse(report('codex', {'session_id': 'native-secret', 'hook_event_name': 'Stop'}, 'w1:p1', seq, config))
                    self.assertEqual(call.call_count, 1)

    def test_opaque_pane_identifiers_resolve_before_reporting(self):
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / 'config.json'
            config.write_text('{"hosts":[{"socket_path":"/herdr/herdr.sock"}]}')
            for pane in ('w1E:p1', 'wABC:pZ9', 'w1:p1'):
                a = agent();a['pane_id'] = pane
                with patch('observatory.telemetry.rpc', side_effect=[{'pane': a}, {}]) as call:
                    self.assertTrue(report('codex', {'session_id': 'native-secret', 'hook_event_name': 'PreToolUse'}, pane, event()['seq'], config))
                    self.assertEqual(call.call_args_list[0].args[2], {'pane_id': pane})
                    self.assertEqual(call.call_args.args[2]['pane_id'], pane)
            for pane in ('', 'x' * 81, 'w1:p1\n', '/tmp/socket', None, ['w1:p1']):
                with patch('observatory.telemetry.rpc') as call:
                    self.assertFalse(report('codex', {'session_id': 'native-secret', 'hook_event_name': 'PreToolUse'}, pane, event()['seq'], config))
                    call.assert_not_called()

    def test_subagent_hooks_discard_child_identity_and_content(self):
        for name, expected in [('SubagentStart', 'subagent-start'), ('SubagentStop', 'subagent-stop')]:
            view = event_view('codex', {'hook_event_name': name, 'agent_id': 'SECRET', 'agent_type': 'SECRET', 'last_assistant_message': 'SECRET', 'agent_transcript_path': '/SECRET'}, event()['seq'])
            self.assertEqual(view['event'], expected)
            self.assertNotIn('SECRET', json.dumps(view))
            self.assertIn(name, installer.EVENTS)

    def test_codex_numeric_enrichment_and_source_expiry(self):
        seq = event()['seq']
        raw = {'hook_event_name': 'PostToolUse', 'observatory_usage': {'input': 100, 'output_tokens': 20, 'cache_read': 0, 'cache_write': 4, 'context': 120, 'window': 1000, 'usage_seq': seq-1000000, 'usage_source': 'codex-rollout', 'secret': 'PRIVATE'}}
        view = event_view('codex', raw, seq)
        self.assertEqual(view['input'],100)
        self.assertEqual(view['cache_read'],0)
        self.assertNotIn('PRIVATE',json.dumps(view))
        raw['observatory_usage']['usage_seq'] = seq-121000000
        view = event_view('codex', raw, seq)
        self.assertIsNone(view['input'])
        self.assertEqual(view['event'],'tool-end')

    def test_pi_native_path_binding(self):
        a = agent('pi');a['agent_session'].update(kind='path', value='/private/session.jsonl')
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / 'config.json';config.write_text('{"hosts":[{"socket_path":"/herdr/herdr.sock"}]}')
            raw = {**event(), 'session_id': 'id', 'session_path': '/private/session.jsonl'}
            with patch('observatory.telemetry.rpc', side_effect=[{'pane': a}, {}]) as call:
                self.assertTrue(report('pi', raw, 'w1:p1', raw['seq'], config))
                self.assertNotIn('/private/', json.dumps(call.call_args.args[2]))

    def test_unsupported_metrics_and_untrusted_fields(self):
        raw = {**event(), 'hook_event_name': 'PostToolUse', 'input': 500, 'tool_response': {'secret': 'SECRET'}, 'tool_name': 'mcp__private_project__call'}
        result = event_view('codex', raw, raw['seq'])
        self.assertIsNone(result['input'])
        self.assertEqual(result['tool'], 'mcp-tool')
        self.assertEqual(result['result'], 'finished')
        self.assertNotIn('SECRET', json.dumps(result))
        for value in (-1, True, 1.2, '500', 2**64, float('nan')):
            result = telemetry_view({**event(), 'input': value, 'secret': 'SECRET'})
            self.assertIsNone(result['input']);self.assertNotIn('secret', result)
        self.assertIsNone(telemetry_view({**event(), 'context': 200, 'window': 100})['context'])
        self.assertIsNone(telemetry_view({**event(), 'seq': int((time.time()-121)*1e6)}))
        self.assertIsNone(telemetry_view({**event(), 'seq': int((time.time()+1)*1e6)}))
        self.assertIsNone(telemetry_view({**event(), 'phase': 'SECRET'}))

    def test_model_paths_and_custom_tools_never_export(self):
        a = agent()
        a['tokens'] = {'obs_v': '1', 'obs_bind': session_binding(a), 'obs_seq': str(event()['seq']), 'obs_event': 'tool-start', 'obs_phase': 'tool', 'obs_model': '/home/private/model', 'obs_tool': '/private/tool'}
        result = telemetry_from_agent(a)
        self.assertIsNone(result['model']);self.assertEqual(result['tool'], 'custom-tool')
        raw = copy.deepcopy(RAW)
        raw['snapshot']['agents'][0]['telemetry'] = {**event(), 'model': '/home/private/model', 'tool': '/private/tool'}
        app = Observatory({'hosts': [HOST]}, 'personal', lambda _: raw);app.poll(HOST)
        self.assertNotIn('/private', json.dumps(project_work(app.snapshot(), HOST['id'])))
        for model in ('/home/private/model', '~/model', 'C:/private', 'C:\\private\\model', '../model'):
            self.assertIsNone(telemetry_view({**event(), 'model': model})['model'])
        self.assertEqual(telemetry_view({**event(), 'model': 'provider/model-1'})['model'], 'provider/model-1')

    def test_work_filter_and_feed_resanitise(self):
        raw = copy.deepcopy(RAW)
        raw['snapshot']['agents'][0]['telemetry'] = {**event(), 'input': 12, 'secret': 'PRIVATE VALUE'}
        raw['snapshot']['agents'][1]['telemetry'] = {**event(), 'model': 'PRIVATE-MODEL'}
        app = Observatory({'hosts': [HOST]}, 'personal', lambda _: raw);app.poll(HOST)
        feed = project_work(app.snapshot(), HOST['id'])
        text = json.dumps(feed)
        self.assertNotIn('PRIVATE', text)
        self.assertEqual(feed['agents'][0]['technical']['telemetry']['input'], 12)

    def test_rpc_real_unix_roundtrip_and_error(self):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / 'herdr.sock')
            with socket.socket(socket.AF_UNIX) as server:
                server.bind(path);server.listen()
                def serve():
                    with server.accept()[0] as client:
                        request = json.loads(client.recv(4096))
                        client.sendall(json.dumps({'id': request['id'], 'result': {'type': 'ok'}}).encode()+b'\n')
                thread = threading.Thread(target=serve);thread.start()
                self.assertEqual(rpc(path, 'pane.report_metadata', {}), {'type': 'ok'})
                thread.join(timeout=2)


class InstallerTests(unittest.TestCase):
    def test_merge_preserves_native_and_uninstall_only_owned(self):
        original = {'extra': 42, 'hooks': {'SessionStart': [{'matcher': 'startup', 'hooks': [{'type': 'command', 'command': 'native-herdr'}]}], 'Other': []}}
        command = "sh '/home/test user/hooks/codex.sh'"
        updated = installer.merge_hooks(original, command)
        self.assertEqual(installer.merge_hooks(updated, command), updated)
        self.assertEqual(installer.merge_hooks(updated, command, True), original)
        self.assertEqual(original['hooks']['SessionStart'][0]['hooks'][0]['command'], 'native-herdr')

    def test_installer_payload_conflicts_repeat_and_uninstall(self):
        def payload(args, **_kw): return (INSTALL.parent / args[-1].split('/')[-1]).read_bytes()
        with tempfile.TemporaryDirectory() as directory, patch.object(installer.shutil, 'which', return_value=None), patch.object(installer.subprocess, 'check_output', side_effect=payload):
            home = Path(directory)
            paths = installer.install(home, 'test-container')
            first = paths[2].read_text()
            installer.install(home, 'test-container')
            self.assertEqual(paths[2].read_text(), first)
            self.assertNotIn('__CONTAINER__', paths[0].read_text())
            self.assertEqual(paths[0].stat().st_mode & 0o777, 0o700)
            installer.install(home, 'test-container', True)
            self.assertFalse(paths[0].exists());self.assertFalse(paths[1].exists())
            installer.install(home, 'test-container', True)
            paths[1].write_text('foreign extension')
            with self.assertRaises(ValueError): installer.install(home, 'test-container')

    def test_invalid_container_never_executes(self):
        with patch.object(installer.subprocess, 'check_output') as call:
            with self.assertRaises(ValueError): installer.install(Path('/tmp/test'), 'x; bad')
            call.assert_not_called()

class InvalidUsageTimestampTests(unittest.TestCase):
    def test_invalid_supplied_usage_time_cannot_retain_numbers(self):
        from observatory.probe import telemetry_view
        for stamp in (None, True, '123', -1, 9007199254740992):
            value=telemetry_view({'seq':100000000,'event':'tool-start','phase':'tool','input':12,'usage_seq':stamp,'usage_source':'codex-rollout'},100)
            self.assertIsNone(value['input'])
            self.assertIsNone(value['usage_source'])
