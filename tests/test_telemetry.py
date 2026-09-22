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
                self.assertNotIn('obs_input', params['tokens'])
                self.assertEqual(params['tokens']['obs_n0'], ',,,')
                self.assertLessEqual(len(params['tokens']), 16)
                self.assertTrue(all(value is None or len(value)<=80 for value in params['tokens'].values()))
                self.assertNotIn('ttl_ms', params)
                self.assertNotIn('SECRET', json.dumps(params))
                a['tokens'] = params['tokens']
                decoded = telemetry_from_agent(a)
                self.assertEqual(decoded['tool'], 'Bash')
                self.assertEqual(telemetry_from_agent(a, time.time()+3600)['tool'], 'Bash')
                self.assertNotIn('native-secret', json.dumps(decoded))
                a['agent_session']['value'] = 'replacement'
                self.assertIsNone(telemetry_from_agent(a))

    def test_native_display_label_uses_harness_context_percentage(self):
        with tempfile.TemporaryDirectory() as directory:
            config=Path(directory)/'config.json'
            config.write_text(json.dumps({'hosts':[{'socket_path':'/herdr/herdr.sock'}]}))
            raw={'session_id':'native-secret','hook_event_name':'PreToolUse','observatory_usage':{'usage_source':'codex-rollout','usage_seq':event()['seq'],'context':190000,'window':258000,'context_percent':72}}
            with patch('observatory.telemetry.rpc',side_effect=[{'pane':agent()},{}]) as call:
                self.assertTrue(report('codex',raw,'w1:p1',event()['seq'],config))
                self.assertIn('ctx~72%',call.call_args.args[2]['display_agent'])

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

    def test_codex_numeric_enrichment_preserves_reported_source_age(self):
        seq = event()['seq']
        raw = {'hook_event_name': 'PostToolUse', 'observatory_usage': {'input': 100, 'output_tokens': 20, 'cache_read': 0, 'cache_write': 4, 'context': 120, 'window': 1000, 'usage_seq': seq-1000000, 'usage_source': 'codex-rollout', 'secret': 'PRIVATE'}}
        view = event_view('codex', raw, seq)
        self.assertEqual(view['input'],100)
        self.assertEqual(view['cache_read'],0)
        self.assertNotIn('PRIVATE',json.dumps(view))
        raw['observatory_usage']['usage_seq'] = seq-121000000
        view = event_view('codex', raw, seq)
        self.assertEqual(view['input'],100)
        self.assertEqual(view['usage_seq'],seq-121000000)
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
        self.assertIsNotNone(telemetry_view({**event(), 'seq': int((time.time()-121)*1e6)}))
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
        for name in ('Interrupt', 'SessionEnd'):
            self.assertEqual(updated['hooks'][name][-1]['hooks'][0]['timeout'], 3)
        self.assertEqual(updated['hooks']['SessionStart'][-1]['hooks'][0]['timeout'], 9)
        self.assertEqual(installer.merge_hooks(updated, command), updated)
        self.assertEqual(installer.merge_hooks(updated, command, True), original)
        self.assertEqual(original['hooks']['SessionStart'][0]['hooks'][0]['command'], 'native-herdr')

    def test_installer_payload_conflicts_repeat_and_uninstall(self):
        def payload(args, **_kw):
            if '-c' in args: return b'false\n'
            name = args[-1].split('/')[-1]
            root = INSTALL.parent.parent / 'observatory' if name == 'allowances_probe.py' else INSTALL.parent
            return (root / name).read_bytes()
        with tempfile.TemporaryDirectory() as directory, patch.object(installer.shutil, 'which', return_value=None), patch.object(installer.subprocess, 'check_output', side_effect=payload):
            home = Path(directory)
            paths = installer.install(home, 'test-container')
            first = paths[2].read_text()
            installer.install(home, 'test-container')
            self.assertEqual(paths[2].read_text(), first)
            self.assertNotIn('__CONTAINER__', paths[0].read_text())
            self.assertIn("if 'false' != 'true'", paths[3].read_text())
            self.assertTrue(paths[4].exists())
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

class CumulativeTelemetryTests(unittest.TestCase):
    def test_totals_are_allowlisted_and_retain_last_known_source_time(self):
        from observatory.probe import telemetry_view
        raw={'seq':100000000,'event':'tool-start','phase':'tool','usage_seq':99000000,'usage_source':'codex-rollout','total_input':21700000,'total_output':3,'total_cache_read':20000000,'total_uncached_input':1700000,'total_cache_write':0,'compactions':0,'context_percent':70}
        value=telemetry_view(raw,100)
        self.assertEqual(value['total_input'],21700000);self.assertEqual(value['compactions'],0);self.assertEqual(value['context_percent'],70)
        value=telemetry_view({**raw,'seq':220000000},220)
        self.assertEqual(value['total_input'],21700000);self.assertEqual(value['context_percent'],70);self.assertEqual(value['compactions'],0)
    def test_invalid_cumulative_values_remain_unknown(self):
        from observatory.probe import telemetry_view
        value=telemetry_view({'seq':100000000,'event':'tool-start','phase':'tool','total_input':True,'compactions':-1,'context_percent':101},100)
        self.assertIsNone(value['total_input']);self.assertIsNone(value['compactions']);self.assertIsNone(value['context_percent'])

class CumulativeConsistencyTests(unittest.TestCase):
    def test_invalid_context_discards_percentage_too(self):
        from observatory.probe import telemetry_view
        for context, window in [(200,100),(0,0)]:
            result=telemetry_view({'seq':100000000,'event':'tool-start','phase':'tool','context':context,'window':window,'context_percent':70},100)
            self.assertIsNone(result['context_percent'])
    def test_contradictory_cached_input_totals_fail_closed(self):
        from observatory.probe import telemetry_view
        result=telemetry_view({'seq':100000000,'event':'tool-start','phase':'tool','total_input':3,'total_cache_read':4,'total_uncached_input':0,'total_output':2},100)
        for key in ('total_input','total_cache_read','total_uncached_input'): self.assertIsNone(result[key])
        self.assertEqual(result['total_output'],2)

class CompactTelemetryTests(unittest.TestCase):
    def test_v2_atomic_groups_and_legacy_migration(self):
        from observatory.probe import TELEMETRY_V2_GROUPS
        a=agent();seq=event()['seq']
        raw={'seq':str(seq),'event':'tool-start','phase':'tool','v':'2','bind':session_binding(a),'usage_source':'codex-rollout'}
        tokens={'obs_'+key:value for key,value in raw.items()}
        values={'input':0,'usage_seq':seq,'total_input':21700000,'total_output':4200,'total_cache_read':20000000,'total_uncached_input':1700000,'context_percent':70,'compactions':3}
        for index,fields in enumerate(TELEMETRY_V2_GROUPS):tokens['obs_n'+str(index)]=','.join(str(values[key]) if key in values else '' for key in fields)
        a['tokens']={**tokens,'obs_input':'999','obs_total_input':'999'}
        view=telemetry_from_agent(a)
        self.assertEqual(view['input'],0);self.assertEqual(view['total_input'],21700000);self.assertEqual(view['compactions'],3)
        self.assertLessEqual(len(tokens),16);self.assertTrue(all(len(v)<=80 for v in tokens.values()))
        for bad in ({'obs_n0':None},{'obs_n0':'1,2'},{'obs_n0':'1,2,3,-1'},{'obs_n0':'1,2,3,9007199254740992'},{'obs_n0':'1,2,3,NaN'},{'obs_v':'3'}):
            a['tokens']={**tokens,**bad,'obs_input':'999'}
            self.assertIsNone(telemetry_from_agent(a))
        a['tokens']={key:value for key,value in tokens.items() if key!='obs_n3'}
        self.assertIsNone(telemetry_from_agent(a))
    def test_v1_flat_read_compatibility(self):
        a=agent();seq=event()['seq'];a['tokens']={'obs_v':'1','obs_bind':session_binding(a),'obs_seq':str(seq),'obs_event':'tool-start','obs_phase':'tool','obs_input':'0'}
        self.assertEqual(telemetry_from_agent(a)['input'],0)
    def test_maximum_safe_numeric_groups_fit_native_value_limit(self):
        from observatory.probe import TELEMETRY_V2_GROUPS
        for fields in TELEMETRY_V2_GROUPS:
            packed=','.join('9007199254740991' for _ in fields)
            self.assertLessEqual(len(packed),67)

    def test_report_roundtrip_transports_cumulative_numbers_atomically(self):
        a=agent();seq=event()['seq']
        usage={'usage_source':'codex-rollout','usage_seq':seq,'input':0,'total_input':21700000,'total_output':4200,'total_cache_read':20000000,'total_uncached_input':1700000,'compactions':3,'context':185000,'window':258400,'context_percent':70}
        with tempfile.TemporaryDirectory() as directory:
            config=Path(directory)/'config.json';config.write_text(json.dumps({'hosts':[{'socket_path':'/herdr/herdr.sock'}]}))
            with patch('observatory.telemetry.rpc',side_effect=[{'pane':a},{}]) as call:
                self.assertTrue(report('codex',{'session_id':'native-secret','hook_event_name':'PreToolUse','observatory_usage':usage},'w1:p1',seq,config))
                tokens=call.call_args.args[2]['tokens'];self.assertEqual(tokens['obs_v'],'2');self.assertEqual(len(tokens),13)
                self.assertTrue(all(value is None or len(value)<=80 for value in tokens.values()))
                a['tokens']=tokens;decoded=telemetry_from_agent(a)
                for key in ('input','total_input','total_output','compactions','context_percent'):self.assertEqual(decoded[key],usage[key])
