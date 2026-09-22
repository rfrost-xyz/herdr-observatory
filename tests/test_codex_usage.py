import importlib.util
import observatory
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from datetime import datetime,timezone

spec=importlib.util.spec_from_file_location('codex_usage',Path(observatory.__file__).resolve().parents[1]/'hooks/codex_usage.py')
helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)

class UsageReaderTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)/'sessions';self.root.mkdir()
        self.file=self.root/'session.jsonl';self.now=1800000000
        self.raw={'session_id':'example','transcript_path':str(self.file),'hook_event_name':'Stop','tool_input':'SECRET','last_assistant_message':'SECRET'}
        self.header={'type':'session_meta','payload':{'id':'example','cwd':'PRIVATE'}}
        self.event={'type':'event_msg','timestamp':datetime.fromtimestamp(self.now-1,timezone.utc).isoformat(),'payload':{'type':'token_count','info':{'last_token_usage':{'input_tokens':12,'output_tokens':3,'cached_input_tokens':0,'cache_write_input_tokens':4,'total_tokens':15},'total_token_usage':{'total_tokens':99999},'model_context_window':100}}}
        self.write()
    def write(self):self.file.write_text(json.dumps(self.header)+'\n'+json.dumps(self.event)+'\n')
    def read(self):return helper.read_usage(self.raw,self.root,self.now)
    def test_last_response_not_cumulative_and_zero_preserved(self):
        value=self.read();self.assertEqual(value['context'],15);self.assertEqual(value['input'],12);self.assertEqual(value['cache_read'],0);self.assertEqual(value['cache_write'],4);self.assertEqual(value['usage_seq'],int((self.now-1)*1e6))
    def test_enrichment_drops_all_raw_content_and_paths(self):
        with patch.object(helper,'read_usage',return_value=self.read()):value=helper.enrich(self.raw)
        self.assertNotIn('SECRET',json.dumps(value));self.assertNotIn(str(self.file),json.dumps(value));self.assertNotIn('PRIVATE',json.dumps(value))
    def test_mismatch_symlink_traversal_and_wrong_owner_rejected(self):
        self.header['payload']['id']='another';self.write();self.assertEqual(self.read(),{})
        self.header['payload']['id']='example';self.write();link=self.root/'link.jsonl';link.symlink_to(self.file);self.raw['transcript_path']=str(link);self.assertEqual(self.read(),{})
        self.raw['transcript_path']=str(self.root/'..'/'sessions'/'session.jsonl');self.assertEqual(self.read(),{})
        self.raw['transcript_path']=str(self.file)
        with patch.object(helper.os,'getuid',return_value=os.getuid()+1):self.assertEqual(self.read(),{})
        alias=Path(self.temp.name)/'alias';alias.symlink_to(self.root,target_is_directory=True)
        self.assertEqual(helper.read_usage({**self.raw,'transcript_path':str(alias/'session.jsonl')},alias,self.now),{})
    def test_stale_future_partial_and_invalid_numbers(self):
        self.assertEqual(helper.read_usage(self.raw,self.root,self.now+121),{})
        self.assertEqual(helper.read_usage(self.raw,self.root,self.now-2),{})
        self.event['payload']['info']['last_token_usage'].pop('cache_write_input_tokens');self.event['payload']['info']['last_token_usage']['input_tokens']=True;self.write()
        self.assertIsNone(self.read()['cache_write']);self.assertIsNone(self.read()['input'])
        self.file.write_text(json.dumps(self.header)+'\n'+json.dumps(self.event));self.assertEqual(self.read(),{})
    def test_bounded_tail_does_not_scan_back_past_limit(self):
        with self.file.open('a') as stream:stream.write('x'*(helper.TAIL_BYTES+1)+'\n')
        self.assertEqual(self.read(),{})
        with self.file.open('a') as stream:stream.write(json.dumps(self.event)+'\n')
        self.assertEqual(self.read()['input'],12)
    def test_latest_invalid_context_not_clamped(self):
        self.event['payload']['info']['model_context_window']=10;self.write();self.assertIsNone(self.read()['context']);self.assertIsNone(self.read()['window'])

    def test_deeply_nested_header_is_silent_and_ordinary_event_survives(self):
        self.file.write_text('['*2000+'0'+']'*2000+'\n')
        self.assertEqual(self.read(),{})
        with patch.object(helper,'read_usage',return_value={}):
            value=helper.enrich(self.raw)
        self.assertEqual(value['hook_event_name'],'Stop')
        self.assertEqual(value['observatory_usage'],{})
