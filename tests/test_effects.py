import os
from pathlib import Path
import subprocess
import time
import unittest
from unittest.mock import patch
from observatory.effects import lines_for

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
        self.assertLessEqual(len(text),6204)

    def test_full_terminal_page_and_hold(self):
        state=self.state()
        _,text=lines_for(state,time.time(),hold=11)
        self.assertEqual(len(text.splitlines()),44)
        for label in ('THREADS','NOTABLE EVENTS','FX HOLD 11s','allowed'):
            self.assertIn(label,text)

    def test_default_hold_and_event_area(self):
        _,text=lines_for(self.state(),time.time())
        self.assertIn('FX HOLD 120s',text)
        self.assertNotIn('rev=',text)
