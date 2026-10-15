import copy
import unittest
from analysis.baseline_report import reconcile


class BaselineReportTests(unittest.TestCase):
    def setUp(self):
        self.observed={'pseudo_iteration':300,'port_flow_m3_s':{'inlet1':-.5,'underflow':.49,'overflow':.01}}
        self.native={'time':300,'patches':{n:{'signed_flow_m3_s':q} for n,q in self.observed['port_flow_m3_s'].items()},
            'hydraulic_loss_j_per_treated_m3':100,'rho_kg_m3':998,'gravity_m_s2':[0,0,0]}

    def test_energy_requires_same_saved_state(self):
        self.assertEqual(reconcile(self.observed,self.native),100)
        bad=copy.deepcopy(self.native);bad['time']=250
        with self.assertRaises(ValueError):reconcile(self.observed,bad)
        bad=copy.deepcopy(self.native);bad['patches']['overflow']['signed_flow_m3_s']=.02
        with self.assertRaises(ValueError):reconcile(self.observed,bad)

    def test_units_and_nonfinite_energy_fail_closed(self):
        for k,v in [('hydraulic_loss_j_per_treated_m3',float('nan')),('rho_kg_m3',1),('gravity_m_s2',[0,0,9.81])]:
            bad=copy.deepcopy(self.native);bad[k]=v
            with self.assertRaises(ValueError):reconcile(self.observed,bad)
