import http.client
import json
from pathlib import Path
import tempfile
import threading
import unittest
from unittest.mock import patch

from dev_server import Model, Handler, ThreadingHTTPServer


class ModelTests(unittest.TestCase):
    def test_order_is_atomic_and_preserves_identity(self):
        model = Model()
        original = [n.copy() for n in model.nodes]
        for ids in [[101,101,103],[101,102],[101,102,999],[True,102,103]]:
            with self.assertRaises(ValueError):
                model.api('PUT','/api/v1/nodes/order',dict(node_ids=ids))
            self.assertEqual(model.nodes,original)
        model.api('PUT','/api/v1/nodes/order',dict(node_ids=[103,101,102]))
        self.assertEqual([n['id'] for n in model.view()],[103,101,102])
        self.assertEqual([n['channel'] for n in model.view()],[1,2,3])

    def test_config_persists_but_output_does_not(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'state.json'
            model = Model(path)
            model.api('PATCH','/api/v1/nodes/101',dict(name='新名称'))
            model.api('POST','/api/v1/nodes/101/control',dict(action='ON'))
            restored = Model(path)
            self.assertEqual(restored.node(101)['name'],'新名称')
            self.assertFalse(restored.view()[0]['output'])
            path.write_text('{broken')
            with self.assertRaises(ValueError):
                Model(path)
            self.assertEqual(path.read_text(),'{broken')

    def test_pairing_window_limit_and_stable_ids(self):
        model = Model()
        with self.assertRaises(ValueError):
            model.api('POST','/api/v1/demo/nodes',dict(name='测试'))
        model.api('POST','/api/v1/pairing/start',{})
        for i in range(5):
            model.api('POST','/api/v1/demo/nodes',dict(name=f'测试{i}'))
        self.assertEqual(len({n['id'] for n in model.nodes}),8)
        with self.assertRaises(ValueError):
            model.api('POST','/api/v1/demo/nodes',dict(name='第九个'))
        model.pair_until = 0
        with self.assertRaises(ValueError):
            model.api('POST','/api/v1/demo/nodes',dict(name='过期'))

    def test_disabling_clears_output_and_blocks_control(self):
        model = Model()
        model.api('POST','/api/v1/nodes/101/control',dict(action='ON'))
        model.api('PATCH','/api/v1/nodes/101',dict(enabled=False))
        self.assertFalse(model.view()[0]['output'])
        with self.assertRaises(ValueError):
            model.api('POST','/api/v1/nodes/101/control',dict(action='TOGGLE'))

    def test_pulse_expires_without_real_sleep(self):
        model = Model()
        with patch('dev_server.time.monotonic',return_value=100):
            model.api('POST','/api/v1/nodes/101/control',dict(action='PULSE',duration_ms=100))
            self.assertTrue(model.view()[0]['output'])
        with patch('dev_server.time.monotonic',return_value=100.1):
            self.assertFalse(model.view()[0]['output'])
        for invalid in [0,60001,True,'500']:
            with self.assertRaises(ValueError):
                model.api('POST','/api/v1/nodes/101/control',dict(action='PULSE',duration_ms=invalid))

    def test_storage_failures_roll_back_config(self):
        model = Model()
        original = [n.copy() for n in model.nodes]
        with patch.object(model,'save',side_effect=OSError('disk full')):
            with self.assertRaises(OSError):
                model.api('PATCH','/api/v1/nodes/101',dict(name='不会保存'))
            with self.assertRaises(OSError):
                model.api('PUT','/api/v1/nodes/order',dict(node_ids=[103,102,101]))
            with self.assertRaises(OSError):
                model.api('DELETE','/api/v1/nodes/101',dict(confirm_id=101))
            model.api('POST','/api/v1/pairing/start',{})
            with self.assertRaises(OSError):
                model.api('POST','/api/v1/demo/nodes',dict(name='不会添加'))
        self.assertEqual(model.nodes,original)

    def test_invalid_inputs_and_guarded_delete(self):
        model = Model()
        for body in [dict(name=''),dict(name='x'*33),dict(enabled=1),dict(owner='x'),[]]:
            with self.assertRaises(ValueError):
                model.api('PATCH','/api/v1/nodes/101',body)
        with self.assertRaises(ValueError):
            model.api('DELETE','/api/v1/nodes/101',dict(confirm_id=102))
        model.api('DELETE','/api/v1/nodes/101',dict(confirm_id=101))
        self.assertIsNone(model.node(101))


    def test_unknown_gpio_is_read_only_and_presets_cannot_be_patched(self):
        model = Model()
        unknown = next(n for n in model.view() if n['id'] == 103)
        self.assertEqual(unknown['device_type'], 'unknown')
        self.assertEqual([p['level'] for p in unknown['gpio']], [False, True, None])
        for route, payload in [('control',dict(action='ON')),('rc',dict(action='FORWARD',seq=1))]:
            with self.assertRaises(ValueError):
                model.api('POST',f'/api/v1/nodes/103/{route}',payload)
        with self.assertRaises(KeyError):
            model.api('POST','/api/v1/nodes/103/gpio',dict(pin=0,level=True))
        with self.assertRaises(ValueError):
            model.api('PATCH','/api/v1/nodes/103',dict(device_type='switch'))
        with self.assertRaises(ValueError):
            model.api('POST','/api/v1/nodes/102/control',dict(action='ON'))
        with self.assertRaises(ValueError):
            model.api('POST','/api/v1/nodes/101/rc',dict(action='FORWARD',seq=1))

    def test_rc_latest_frame_stop_watchdog_disable_and_invalid_inputs(self):
        model = Model()
        with patch('dev_server.time.monotonic',return_value=100):
            result = model.api('POST','/api/v1/nodes/102/rc',dict(action='FORWARD',seq=1))
            self.assertEqual(result['throttle'],35)
        with patch('dev_server.time.monotonic',return_value=100.2):
            with self.assertRaises(ValueError):
                model.api('POST','/api/v1/nodes/102/rc',dict(action='BOOST_FORWARD',seq=1))
        with patch('dev_server.time.monotonic',return_value=100.3):
            rc = next(n for n in model.view() if n['id']==102)['rc']
            self.assertEqual((rc['throttle'],rc['steering']),(0,0))
        for seq, action in [(2,'BOOST_FORWARD'),(3,'REVERSE'),(4,'LEFT'),(5,'RIGHT'),(6,'STOP')]:
            model.api('POST','/api/v1/nodes/102/rc',dict(action=action,seq=seq))
        self.assertEqual(model.rc[102]['throttle'],0)
        self.assertEqual(model.rc[102]['steering'],0)
        for payload in [dict(action='UNKNOWN',seq=7),dict(action='FORWARD',seq=True),dict(action=[],seq=7)]:
            with self.assertRaises(ValueError):
                model.api('POST','/api/v1/nodes/102/rc',payload)
        model.api('POST','/api/v1/nodes/102/rc',dict(action='FORWARD',seq=7))
        model.api('PATCH','/api/v1/nodes/102',dict(enabled=False))
        self.assertEqual(model.rc[102]['throttle'],0)
        with self.assertRaises(ValueError):
            model.api('POST','/api/v1/nodes/102/rc',dict(action='FORWARD',seq=8))

    def test_legacy_profiles_migrate_to_unknown_and_v2_preserves_defined_type(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'legacy.json'
            original = dict(version=1,nodes=[dict(id=1,name='Legacy',enabled=True)])
            path.write_text(json.dumps(original))
            model = Model(path)
            self.assertEqual(model.nodes[0]['device_type'],'unknown')
            self.assertEqual(json.loads(path.read_text()),original)
            model.api('POST','/api/v1/pairing/start',{})
            created = model.api('POST','/api/v1/demo/nodes',dict(name='Toy',device_type='rc_basic'))
            self.assertEqual(created['device_type'],'rc_basic')
            self.assertEqual(json.loads(path.read_text())['version'],2)
            restored = Model(path)
            self.assertEqual(restored.node(created['id'])['device_type'],'rc_basic')
            self.assertEqual(restored.view()[1]['rc']['throttle'],0)


class HttpTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ThreadingHTTPServer(('127.0.0.1',0),Handler)
        cls.server.model = Model()
        cls.thread = threading.Thread(target=cls.server.serve_forever,daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()

    def request(self,method,path,body=None,headers=None):
        connection = http.client.HTTPConnection('127.0.0.1',self.server.server_port,timeout=3)
        connection.request(method,path,body,headers or {})
        response = connection.getresponse()
        data = response.read()
        result = (response.status,response.getheaders(),data)
        connection.close()
        return result

    def test_static_assets_and_envelope(self):
        for path in ['/','/style.css','/app.js']:
            status,headers,data = self.request('GET',path)
            self.assertEqual(status,200)
            self.assertTrue(data)
            self.assertIn('Content-Security-Policy',dict(headers))
        status,_,raw = self.request('GET','/api/v1/status')
        self.assertEqual(status,200)
        data = json.loads(raw)
        self.assertEqual(set(data),{'ok','data','error'})
        self.assertEqual(data['data']['mode'],'simulation')
        self.assertFalse(data['data']['firmware_connected'])
        self.assertEqual(self.request('GET','/../Cargo.toml')[0],404)

    def test_authenticated_same_origin_mutations(self):
        path = '/api/v1/pairing/start'
        headers = {'Content-Type':'application/json'}
        self.assertEqual(self.request('POST',path,'{}',headers)[0],403)
        headers['X-NowCtrl-Token'] = self.server.model.token
        headers['Origin'] = 'https://other.invalid'
        self.assertEqual(self.request('POST',path,'{}',headers)[0],403)
        del headers['Origin']
        self.assertEqual(self.request('POST',path,'{}',headers)[0],200)
        self.assertEqual(self.request('POST',path,'[]',headers)[0],400)
        self.assertEqual(self.request('POST',path,'{broken',headers)[0],400)
        self.assertEqual(self.request('POST',path,'x'*4097,headers)[0],400)
        headers['Host'] = 'attacker.invalid'
        self.assertEqual(self.request('POST',path,'{}',headers)[0],403)


if __name__ == '__main__':
    unittest.main()
