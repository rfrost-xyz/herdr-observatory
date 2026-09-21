import os
from pathlib import Path
import subprocess
import time
import unittest
from unittest.mock import patch
from observatory.effects import lines_for, parse_frames, TextEffects

class EffectsTests(unittest.TestCase):
    def state(self):
        return {'interval':5,'hosts':[{'id':'node','online':True,'sampled_at':time.time(), 'agents':[{'host':'node','project':'allowed','status':'working','harness':'agent','title':'checking tests','technical':{}}]}]}

    def test_stale_and_offline_never_render(self):
        state=self.state();state['hosts'][0]['sampled_at']=1
        self.assertEqual(lines_for(state,time.time())[0],'')
        state['hosts'][0]['sampled_at']=time.time();state['hosts'][0]['online']=False
        self.assertEqual(lines_for(state,time.time())[0],'')

    def test_controls_removed_and_input_bounded(self):
        state=self.state();state['hosts'][0]['agents'][0]['title']='\x1b[31m'+ 'x'*1000
        _,text=lines_for(state,time.time())
        self.assertNotIn('\x1b',text)
        self.assertLessEqual(len(text),605)

    def test_invalid_frames_rejected(self):
        for raw in (b'5\na\n',b'3\n\x1bxx\n',b'9000\n',b'x\n',b'101\n'+b'a'*101+b'\n'):
            with self.assertRaises(ValueError):parse_frames(raw)

    def test_cache_and_plain_failure(self):
        state=self.state();engine=TextEffects()
        with patch('observatory.effects.subprocess.run',side_effect=OSError) as run:
            result=engine.snapshot(state)
            self.assertEqual(result['effect'],'plain')
            self.assertIn('allowed',result['text'])
            self.assertEqual(engine.snapshot(state),result)
            self.assertEqual(run.call_count,1)

    def test_actual_library_effects(self):
        binary=os.environ.get('OBSERVATORY_TEXT_RENDERER')
        if not binary or not Path(binary).exists():self.skipTest('ttfx adapter not configured')
        for effect in ('decrypt','vhstape','crumble'):
            result=subprocess.run([binary,effect],input=('\n'.join(('node WORKING rev=42 seq=17 '+str(i)+' '+'x'*90)[:100] for i in range(6))).encode(),capture_output=True,check=True,timeout=2)
            frames=parse_frames(result.stdout)
            self.assertGreater(len(frames),1)
            self.assertLessEqual(len(frames),120)
            self.assertGreater(len(set(frames)),1)

    def test_timeout_falls_back_and_source_loss_invalidates_cache(self):
        state=self.state();engine=TextEffects()
        with patch('observatory.effects.subprocess.run',side_effect=subprocess.TimeoutExpired('renderer',2)):
            self.assertEqual(engine.snapshot(state)['effect'],'plain')
        state['hosts'][0]['online']=False
        result=engine.snapshot(state)
        self.assertEqual(result['key'],'')
        self.assertEqual(result['frames'],[])
        self.assertNotIn('allowed',result['text'])
