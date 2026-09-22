import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from observatory.allowances_probe import summarise
from observatory.allowances import Allowances, sanitise, receive, read_cache, validate_config

NOW = 1790000000
KEY = hashlib.sha256(b'observatory-codex-account-v1:example-account').hexdigest()


def source():
    return {'accountId': 'example-account', 'rateLimits': {'planType': 'pro', 'limitId': 'codex',
            'primary': {'usedPercent': 74, 'windowDurationMins': 10080, 'resetsAt': NOW + 3600}},
            'rateLimitResetCredits': {'availableCount': 2, 'credits': [
                {'id': 'private-credit', 'title': 'private-title', 'status': 'available',
                 'resetType': 'codexRateLimits', 'expiresAt': NOW + 7200}]}}


class AllowanceTests(unittest.TestCase):
    def test_supported_weekly_window_identity_and_credit_count(self):
        result = summarise(source(), NOW)
        self.assertEqual(result['account_key'], KEY)
        self.assertEqual(result['weekly_remaining'], 26)
        self.assertEqual(result['reset_count'], 2)  # details may be capped
        self.assertIsNone(result['reset_expires_at'])  # list is capped; earliest unknown
        complete = source(); complete['rateLimitResetCredits']['availableCount'] = 1
        self.assertEqual(summarise(complete, NOW)['reset_expires_at'], NOW + 7200)
        encoded = json.dumps(result)
        for private in ('example-account', 'private-credit', 'private-title'):
            self.assertNotIn(private, encoded)

    def test_secondary_weekly_and_missing_identity(self):
        raw = source()
        raw['rateLimits']['secondary'] = raw['rateLimits'].pop('primary')
        self.assertEqual(summarise(raw, NOW)['weekly_remaining'], 26)
        raw.pop('accountId')
        self.assertIsNone(summarise(raw, NOW))

    def test_other_bucket_not_mislabelled_weekly(self):
        raw = source(); raw['rateLimits']['limitId'] = 'special-model'
        self.assertIsNone(summarise(raw, NOW))
        raw = source(); raw['rateLimits']['primary']['windowDurationMins'] = 300
        self.assertIsNone(summarise(raw, NOW)['weekly_remaining'])

    def test_unknown_distinct_from_zero(self):
        raw = source(); raw['rateLimitResetCredits'] = None
        self.assertIsNone(summarise(raw, NOW)['reset_count'])
        raw['rateLimitResetCredits'] = {'availableCount': 0, 'credits': []}
        self.assertEqual(summarise(raw, NOW)['reset_count'], 0)
        raw['rateLimits']['primary']['usedPercent'] = True
        self.assertIsNone(summarise(raw, NOW)['weekly_remaining'])

    def test_elapsed_reset_never_implies_refill_or_available_credit(self):
        row = summarise(source(), NOW)
        row['weekly_resets_at'] = NOW + 10
        row['reset_expires_at'] = NOW + 20
        later = sanitise(row, NOW + 30)
        self.assertIsNone(later['weekly_remaining'])
        self.assertIsNone(later['reset_count'])
        self.assertIsNone(sanitise(row, NOW + 601))
        self.assertIsNone(sanitise(row, NOW - 1))

    def test_mapping_validation(self):
        validate_config({'accounts': {KEY: 'Personal'}, 'publish': False})
        for value in ({'accounts': {'not-hash': 'Personal'}}, {'accounts': {KEY: 'iapetus'}},
                      {'accounts': {KEY: 'Personal'}, 'publish': 'yes'},
                      {'accounts': {KEY: 'Personal'}, 'sources': [{'target': '-x', 'container': 'safe'}]}):
            with self.assertRaises(ValueError): validate_config(value)

    def test_cache_mapping_atomic_dedupe_and_public_privacy(self):
        config = {'accounts': {KEY: 'Personal'}}
        row = summarise(source(), NOW)
        with tempfile.TemporaryDirectory() as temp:
            path = str(Path(temp) / 'accounts.json')
            self.assertTrue(receive(row, config, path, NOW))
            self.assertFalse(receive(row, config, path, NOW))
            other = dict(row, account_key='a'*64)
            self.assertFalse(receive(other, config, path, NOW))
            self.assertEqual(len(read_cache(path, NOW)), 1)
            instance = Allowances(config)
            with patch('observatory.allowances.time.time', return_value=NOW), patch('observatory.allowances.read_cache', return_value=[row]):
                public = instance.snapshot([dict(row, sampled_at=NOW - 1)])
            self.assertEqual(len(public), 1)
            self.assertEqual(public[0]['label'], 'Personal')
            self.assertNotIn(KEY, json.dumps(public))
            self.assertNotIn('account_key', public[0])

    def test_configured_account_without_fresh_data_is_unavailable(self):
        instance = Allowances({'accounts': {KEY: 'Personal'}})
        with patch('observatory.allowances.read_cache', return_value=[]):
            self.assertEqual(instance.snapshot()[0]['available'], False)
            self.assertIsNone(instance.snapshot()[0]['weekly_remaining'])

class ProbeProcessTests(unittest.TestCase):
    def executable(self, directory, body):
        script = Path(directory) / 'codex'
        script.write_text('#!/usr/bin/env python3\n' + body)
        script.chmod(0o700)
        return script

    def test_only_read_rpc_and_no_raw_identity_output(self):
        import os
        from observatory.allowances_probe import read_account
        with tempfile.TemporaryDirectory() as directory:
            record = str(Path(directory) / 'methods.json')
            body = ('import sys,json\nmethods=[]\nfor line in sys.stdin:\n'
                    ' x=json.loads(line);methods.append(x["method"])\n'
                    ' if x.get("id")==1: print(json.dumps({"id":1,"result":{}}),flush=True)\n'
                    ' if x.get("id")==2:\n'
                    f'  open({record!r},"w").write(json.dumps(methods))\n'
                    f'  print(json.dumps({{"id":2,"result":{source()!r}}}),flush=True)\n')
            self.executable(directory, body)
            with patch.dict(os.environ, {'PATH': directory + os.pathsep + os.environ['PATH']}), patch('observatory.allowances_probe.time.time', return_value=NOW):
                row = read_account(timeout=2)
            self.assertEqual(row['account_key'], KEY)
            self.assertEqual(json.loads(Path(record).read_text()), ['initialize', 'initialized', 'account/rateLimits/read'])

    def test_partial_frame_and_oversized_output_are_bounded(self):
        import os
        import time
        from observatory.allowances_probe import read_account
        for body in ('import sys,time\nsys.stdout.write("{");sys.stdout.flush();time.sleep(10)\n',
                     'import sys\nsys.stdout.write("x"*300000);sys.stdout.flush()\n'):
            with tempfile.TemporaryDirectory() as directory:
                self.executable(directory, body)
                start = time.monotonic()
                with patch.dict(os.environ, {'PATH': directory + os.pathsep + os.environ['PATH']}):
                    self.assertIsNone(read_account(timeout=.2))
                self.assertLess(time.monotonic() - start, 1)

    def test_remote_cache_command_bound(self):
        import sys
        from observatory.allowances import bounded_command
        self.assertIsNone(bounded_command([sys.executable, '-c', 'print("x"*20000)']))
        self.assertEqual(bounded_command([sys.executable, '-c', 'print("[]")']), b'[]\n')


class HookAllowanceTests(unittest.TestCase):
    def test_disabled_by_default_throttled_and_no_identity_storage(self):
        import sys
        import types
        from observatory import __file__ as package
        helper = Path(package).parent.parent / 'hooks' / 'codex_usage.py'
        with tempfile.TemporaryDirectory() as directory:
            namespace = {'__name__': 'synthetic_hook', '__file__': str(Path(directory) / 'codex_usage.py')}
            exec(compile(helper.read_text(), str(helper), 'exec'), namespace)
            with patch('subprocess.run') as run:
                namespace['refresh_allowances']()
                run.assert_not_called()
            exec(compile(helper.read_text().replace('__ALLOWANCES__', 'true'), str(helper), 'exec'), namespace)
            module = types.ModuleType('allowances_probe')
            module.read_account = lambda **kw: summarise(source(), NOW)
            with patch.dict(sys.modules, {'allowances_probe': module}), patch('subprocess.run') as run:
                namespace['refresh_allowances']()
                namespace['refresh_allowances']()
                self.assertEqual(run.call_count, 1)
                payload = json.loads(run.call_args.kwargs['input'])
                self.assertEqual(payload['account_key'], KEY)
            text = (Path(directory) / 'allowances-refresh.lock').read_text()
            self.assertNotIn(KEY, text)
            self.assertNotIn('example-account', text)

class AllowancePrivacyTests(unittest.TestCase):
    def test_export_revalidates_current_account_mapping(self):
        from observatory.allowances import export_rows
        row = summarise(source(), NOW)
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / 'config.json'
            config.write_text(json.dumps({'allowances': {'accounts': {'b'*64: 'Work'}}}))
            with patch('observatory.allowances.read_cache', return_value=[row]):
                self.assertEqual(export_rows(str(config)), [])
                config.write_text(json.dumps({'allowances': {'accounts': {KEY: 'Personal'}}}))
                self.assertEqual(export_rows(str(config)), [row])
                config.write_text('{}')
                self.assertEqual(export_rows(str(config)), [])

    def test_feed_allowances_are_separate_opt_in_and_revalidated(self):
        from observatory.feed import validate_feed, project_work
        row = summarise(source(), NOW)
        row['email'] = 'private@example.test'
        raw = {'schema': 'herdr-work-v1', 'profile': 'work', 'host_id': 'office',
               'captured_at': NOW, 'agents': [], 'online': True, 'allowances': [row]}
        with patch('observatory.allowances.time.time', return_value=NOW), patch('observatory.feed.time.time', return_value=NOW):
            validated = validate_feed(raw)
            self.assertNotIn('email', validated['allowances'][0])
            snap = {'hosts': [{'id': 'office', 'sampled_at': NOW, 'online': True, 'agents': [], 'metrics': None}],
                    'theme': None, 'allowances': [row]}
            self.assertEqual(project_work(snap, 'office')['allowances'], [])

    def test_malformed_plan_and_timestamp_remain_safe(self):
        for value in ([], {}, True):
            raw = source(); raw['rateLimits']['planType'] = value
            self.assertIsNone(summarise(raw, NOW)['plan'])
        row = summarise(source(), NOW)
        self.assertIsNone(sanitise(dict(row, sampled_at=10**1000), NOW))
        self.assertIsNone(sanitise(dict(row, plan=[]), NOW)['plan'])

    def test_hook_timestamp_file_refuses_symlink_and_oversize(self):
        import sys
        import types
        from observatory import __file__ as package
        helper = Path(package).parent.parent / 'hooks' / 'codex_usage.py'
        with tempfile.TemporaryDirectory() as directory:
            namespace = {'__name__': 'synthetic_hook', '__file__': str(Path(directory) / 'codex_usage.py')}
            exec(compile(helper.read_text().replace('__ALLOWANCES__', 'true'), str(helper), 'exec'), namespace)
            module = types.ModuleType('allowances_probe')
            from unittest.mock import Mock
            module.read_account = Mock()
            target = Path(directory) / 'other'
            target.write_text('x')
            lock = Path(directory) / 'allowances-refresh.lock'
            lock.symlink_to(target)
            with patch.dict(sys.modules, {'allowances_probe': module}):
                with self.assertRaises(OSError): namespace['refresh_allowances']()
                lock.unlink(); lock.write_text('0'*65)
                namespace['refresh_allowances']()
                module.read_account.assert_not_called()
            self.assertEqual(target.read_text(), 'x')


class CodexBinaryTests(unittest.TestCase):
    def test_path_binary_has_priority(self):
        from observatory.allowances_probe import codex_binary
        with patch('observatory.allowances_probe.shutil.which', return_value='/trusted/codex'):
            self.assertEqual(codex_binary(), '/trusted/codex')

    def test_existing_user_shim_fallback_and_missing_binary(self):
        from observatory.allowances_probe import codex_binary, read_account
        with tempfile.TemporaryDirectory() as directory, patch('observatory.allowances_probe.shutil.which', return_value=None), patch('observatory.allowances_probe.Path.home', return_value=Path(directory)):
            with patch('observatory.allowances_probe.subprocess.Popen') as process:
                self.assertIsNone(read_account())
                process.assert_not_called()
            shim = Path(directory) / '.local/bin/codex'
            shim.parent.mkdir(parents=True)
            actual = Path(directory) / 'installed-codex'
            actual.write_text('#!/bin/sh\nexit 0\n')
            actual.chmod(0o700)
            shim.symlink_to(actual)
            self.assertEqual(codex_binary(), str(shim))
            actual.chmod(0o600)
            self.assertIsNone(codex_binary())
