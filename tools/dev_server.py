#!/usr/bin/env python3
"""Loopback-only management UI and virtual-node API. No ESP/radio access."""
import argparse
import hmac
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import secrets
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
MAX_NODES = 8
PROFILES = ('switch', 'rc_basic', 'unknown')
RC_ACTIONS = {'STOP': (0, 0), 'FORWARD': (35, 0), 'BOOST_FORWARD': (65, 0),
              'REVERSE': (-30, 0), 'LEFT': (0, -100), 'RIGHT': (0, 100)}


class Model:
    def __init__(self, path=None):
        self.path = Path(path) if path else None
        self.lock = threading.RLock()
        self.token = secrets.token_urlsafe(32)
        self.started = time.monotonic()
        self.pair_until = 0
        self.events = []
        self.event_id = 0
        self.request_id = 0
        self.outputs = {}
        self.rc = {}
        self.nodes = [dict(id=101, name='工作台灯', enabled=True, device_type='switch'),
                      dict(id=102, name='模玩遥控', enabled=True, device_type='rc_basic'),
                      dict(id=103, name='未定义设备', enabled=True, device_type='unknown')]
        if self.path and self.path.exists():
            data = json.loads(self.path.read_text())
            if not isinstance(data, dict) or data.get('version') not in (1, 2):
                raise ValueError('Unsupported demo storage version')
            nodes = data['nodes']
            if not isinstance(nodes, list) or len(nodes) > MAX_NODES:
                raise ValueError('Invalid stored nodes')
            ids = set()
            migrated = []
            for node in nodes:
                if data['version'] == 1 and isinstance(node, dict):
                    node = dict(node, device_type='unknown')

                if (not isinstance(node, dict) or set(node) != {'id','name','enabled','device_type'} or type(node.get('id')) is not int
                        or not 0 < node['id'] <= 0xffffffff or node['id'] in ids
                        or not self.valid_name(node.get('name')) or type(node.get('enabled')) is not bool
                        or node.get('device_type') not in PROFILES):
                    raise ValueError('Invalid stored node')
                ids.add(node['id'])
                migrated.append(node)
            self.nodes = migrated
        self.event('system', '软件模拟已启动；没有连接真实设备')

    @staticmethod
    def valid_name(value):
        return isinstance(value, str) and 1 <= len(value.strip()) <= 32 and all(ord(c) >= 32 for c in value)

    def event(self, kind, message):
        self.event_id += 1
        self.events.append(dict(id=self.event_id, time=time.strftime('%H:%M:%S'), kind=kind, message=message))
        self.events = self.events[-100:]

    def save(self):
        if not self.path:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temp = self.path.with_suffix('.tmp')
        try:
            with temp.open('w') as stream:
                json.dump(dict(version=2, nodes=self.nodes), stream, ensure_ascii=False)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temp, self.path)
        finally:
            temp.unlink(missing_ok=True)

    def node(self, node_id):
        return next((n for n in self.nodes if n['id'] == node_id), None)

    def view(self):
        now = time.monotonic()
        nodes = []
        for i, node in enumerate(self.nodes):
            output, until = self.outputs.get(node['id'], (False, None))
            if until is not None and now >= until:
                output = False
                self.outputs[node['id']] = (False, None)
                self.event('control', f"{node['name']}：模拟脉冲结束，虚拟输出 OFF")
            rc = self.rc.get(node['id'], dict(seq=0, action='STOP', throttle=0, steering=0, deadline=None))
            if rc['deadline'] is not None and now >= rc['deadline']:
                rc = dict(rc, action='STOP', throttle=0, steering=0, deadline=None)
                self.rc[node['id']] = rc
                self.event('control', f"{node['name']}：模拟 RC 超时，油门与转向归零")
            nodes.append(dict(**node, channel=i+1, online=node['enabled'], output=output, simulated=True,
                              rc={key: rc[key] for key in ('seq','action','throttle','steering')},
                              gpio=[dict(pin=0, level=False),dict(pin=1, level=True),dict(pin=2, level=None)]
                              if node['device_type'] == 'unknown' else []))
        return nodes

    def api(self, method, path, body=None):
        with self.lock:
            if method == 'GET':
                if path == '/api/v1/status':
                    return dict(mode='simulation', firmware_connected=False, rf_channel=6,
                                version='0.1.0-dev', max_nodes=MAX_NODES,
                                uptime_s=int(time.monotonic()-self.started),
                                pairing_remaining_s=max(0, int(self.pair_until-time.monotonic())),
                                mutation_token=self.token)
                if path == '/api/v1/nodes':
                    return self.view()
                if path == '/api/v1/demo/events':
                    self.view()
                    return self.events
                raise KeyError('接口不存在')
            body = {} if body is None else body
            if not isinstance(body, dict):
                raise ValueError('请求必须是 JSON 对象')
            if method == 'POST' and path == '/api/v1/pairing/start':
                self.pair_until = time.monotonic()+60
                self.event('pairing', '模拟配对窗口已打开，持续 60 秒')
                return dict(remaining_s=60)
            if method == 'POST' and path == '/api/v1/demo/nodes':
                if time.monotonic() >= self.pair_until:
                    raise ValueError('请先开启模拟配对窗口')
                if len(self.nodes) >= MAX_NODES:
                    raise ValueError('节点数量已达 8 个上限')
                name = body.get('name')
                if not self.valid_name(name):
                    raise ValueError('名称需为 1–32 个可见字符')
                node_id = secrets.randbelow(0xffffffff)+1
                while self.node(node_id):
                    node_id = secrets.randbelow(0xffffffff)+1
                device_type = body.get('device_type', 'unknown')
                if device_type not in PROFILES:
                    raise ValueError('不支持的设备类型')
                self.nodes.append(dict(id=node_id, name=name.strip(), enabled=True, device_type=device_type))
                try:
                    self.save()
                except OSError:
                    self.nodes.pop()
                    raise
                self.event('pairing', f'{name.strip()}：模拟节点已添加')
                return self.view()[-1]
            if method == 'PUT' and path == '/api/v1/nodes/order':
                ids = body.get('node_ids')
                if (not isinstance(ids, list) or any(type(i) is not int for i in ids)
                        or len(ids) != len(self.nodes) or len(set(ids)) != len(ids)
                        or set(ids) != {n['id'] for n in self.nodes}):
                    raise ValueError('排序必须恰好包含全部节点 ID，且不可重复')
                old = self.nodes
                self.nodes = [self.node(i) for i in ids]
                try:
                    self.save()
                except OSError:
                    self.nodes = old
                    raise
                self.event('config', '逻辑通道顺序已保存；节点身份保持不变')
                return self.view()
            parts = path.strip('/').split('/')
            if len(parts) not in (4, 5) or parts[:3] != ['api', 'v1', 'nodes']:
                raise KeyError('接口不存在')
            try:
                node_id = int(parts[3])
            except ValueError as error:
                raise KeyError('节点不存在') from error
            node = self.node(node_id)
            if node is None:
                raise KeyError('节点不存在')
            if len(parts) == 4 and method == 'PATCH':
                if not body or not set(body) <= {'name', 'enabled'}:
                    raise ValueError('仅支持名称和启用状态')
                if 'name' in body and not self.valid_name(body['name']):
                    raise ValueError('名称需为 1–32 个可见字符')
                if 'enabled' in body and type(body['enabled']) is not bool:
                    raise ValueError('enabled 必须是布尔值')
                old = node.copy()
                node.update(body)
                node['name'] = node['name'].strip()
                try:
                    self.save()
                except OSError:
                    node.clear()
                    node.update(old)
                    raise
                if not node['enabled']:
                    self.outputs[node_id] = (False, None)
                    previous = self.rc.get(node_id, {'seq':0})
                    self.rc[node_id] = dict(seq=previous['seq'], action='STOP', throttle=0, steering=0, deadline=None)
                self.event('config', f"{node['name']}：设置已保存")
                return next(n for n in self.view() if n['id'] == node_id)
            if len(parts) == 4 and method == 'DELETE':
                if body.get('confirm_id') != node_id or type(body.get('confirm_id')) is not int:
                    raise ValueError('删除需要确认同一节点 ID')
                old = self.nodes
                self.nodes = [n for n in self.nodes if n['id'] != node_id]
                try:
                    self.save()
                except OSError:
                    self.nodes = old
                    raise
                self.outputs.pop(node_id, None)
                self.rc.pop(node_id, None)
                self.event('config', f"{node['name']}：模拟节点已移除")
                return dict(id=node_id)
            if len(parts) == 5 and parts[4] == 'rc' and method == 'POST':
                if node['device_type'] != 'rc_basic':
                    raise ValueError('该设备不支持模玩预设；未定义设备的 GPIO 仅可查看')
                if not node['enabled']:
                    raise ValueError('节点已停用')
                action, seq = body.get('action'), body.get('seq')
                if not isinstance(action, str) or action not in RC_ACTIONS:
                    raise ValueError('不支持的模玩动作')
                if type(seq) is not int or not 0 < seq <= 0xffffffff:
                    raise ValueError('RC 序号无效')
                previous = self.rc.get(node_id, {'seq':0})
                if seq <= previous['seq']:
                    raise ValueError('过期或重复的 RC 更新')
                throttle, steering = RC_ACTIONS[action]
                self.rc[node_id] = dict(seq=seq, action=action, throttle=throttle, steering=steering,
                                        deadline=None if action == 'STOP' else time.monotonic()+0.3)
                if action != previous.get('action'):
                    self.event('control', f"{node['name']}：模拟 RC {action} / 油门 {throttle}% / 转向 {steering}%")
                return dict(node_id=node_id, seq=seq, action=action, throttle=throttle, steering=steering, simulated=True)
            if len(parts) == 5 and parts[4] == 'control' and method == 'POST':
                if node['device_type'] != 'switch':
                    raise ValueError('该设备不支持开关控制；未定义设备的 GPIO 仅可查看')
                if not node['enabled']:
                    raise ValueError('节点已停用')
                action = body.get('action')
                if action not in ('ON','OFF','TOGGLE','PULSE'):
                    raise ValueError('不支持的控制动作')
                duration = body.get('duration_ms', 500)
                if action == 'PULSE' and (type(duration) is not int or not 1 <= duration <= 60000):
                    raise ValueError('模拟脉冲范围为 1–60000 毫秒')
                self.view()
                current = self.outputs.get(node_id, (False, None))[0]
                output = action in ('ON','PULSE') or (action == 'TOGGLE' and not current)
                self.outputs[node_id] = (output, time.monotonic()+duration/1000 if action == 'PULSE' else None)
                self.request_id += 1
                self.event('control', f"{node['name']}：{action} → {'ON' if output else 'OFF'} / 模拟 ACK #{self.request_id}")
                return dict(node_id=node_id, request_id=self.request_id, result='applied', simulated=True)
            raise KeyError('接口不存在')


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.handle_request()

    def do_POST(self):
        self.handle_request()

    def do_PUT(self):
        self.handle_request()

    def do_PATCH(self):
        self.handle_request()

    def do_DELETE(self):
        self.handle_request()

    def respond(self, status, data):
        raw = json.dumps(data, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header('Content-Type','application/json; charset=utf-8')
        self.send_header('Content-Length',str(len(raw)))
        self.send_header('Cache-Control','no-store')
        self.end_headers()
        self.wfile.write(raw)

    def handle_request(self):
        try:
            host = self.headers.get('Host','')
            if host not in (f'127.0.0.1:{self.server.server_port}', f'localhost:{self.server.server_port}'):
                self.respond(403,dict(ok=False,data=None,error='Host 不被允许'))
                return
            if self.path.startswith('/api/'):
                body = None
                if self.command != 'GET':
                    origin = self.headers.get('Origin')
                    if origin and origin != 'http://'+host:
                        self.respond(403,dict(ok=False,data=None,error='跨来源修改被拒绝'))
                        return
                    if not hmac.compare_digest(self.headers.get('X-NowCtrl-Token',''),self.server.model.token):
                        self.respond(403,dict(ok=False,data=None,error='修改令牌无效，请刷新页面'))
                        return
                    length = int(self.headers.get('Content-Length','0'))
                    if not 0 < length <= 4096 or self.headers.get_content_type() != 'application/json':
                        raise ValueError('请求需要 JSON，且不得超过 4096 字节')
                    body = json.loads(self.rfile.read(length))
                data = self.server.model.api(self.command, self.path, body)
                self.respond(200,dict(ok=True,data=data,error=None))
                return
            if self.command != 'GET':
                raise KeyError('页面不存在')
            filename = {'/':'index.html','/index.html':'index.html','/app.js':'app.js','/style.css':'style.css'}.get(self.path.split('?')[0])
            if not filename:
                raise KeyError('页面不存在')
            data = (ROOT/'web'/filename).read_bytes()
            self.send_response(200)
            self.send_header('Content-Type', {'html':'text/html; charset=utf-8','js':'text/javascript; charset=utf-8','css':'text/css; charset=utf-8'}[filename.split('.')[-1]])
            self.send_header('Content-Length',str(len(data)))
            self.send_header('Cache-Control','no-store')
            self.send_header('X-Content-Type-Options','nosniff')
            self.send_header('Content-Security-Policy',"default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'")
            self.end_headers()
            self.wfile.write(data)
        except KeyError as error:
            self.respond(404,dict(ok=False,data=None,error=str(error).strip("'")))
        except (ValueError, json.JSONDecodeError) as error:
            self.respond(400,dict(ok=False,data=None,error=str(error)))
        except OSError:
            self.respond(500,dict(ok=False,data=None,error='本地存储失败，修改未保存'))

    def log_message(self, *args):
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port',type=int,default=8080)
    parser.add_argument('--state',default=str(ROOT/'target/web-demo.json'))
    args = parser.parse_args()
    server = ThreadingHTTPServer(('127.0.0.1',args.port),Handler)
    server.model = Model(args.state)
    print(f'NowCtrl 软件模拟管理页：http://127.0.0.1:{server.server_port}（无真实设备）',flush=True)
    stopped = threading.Event()
    def watchdog():
        while not stopped.wait(0.05):
            with server.model.lock:
                server.model.view()
    worker = threading.Thread(target=watchdog, daemon=True)
    worker.start()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        stopped.set()
        worker.join(timeout=1)
        server.server_close()


if __name__ == '__main__':
    main()
