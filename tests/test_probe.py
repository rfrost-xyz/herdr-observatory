import json
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from observatory import probe
from test_core import SNAP


class ProbeTests(unittest.TestCase):
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
