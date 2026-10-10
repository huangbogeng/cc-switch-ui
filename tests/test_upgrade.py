"""Real CLI installer upgrade/rollback tests; every process uses an isolated HOME."""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import sqlite3
import subprocess
import tarfile
import tempfile
import time
import threading
import unittest
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--new-binary', required=True)
parser.add_argument('--old-binary', required=True)
args, remaining = parser.parse_known_args()
NEW = Path(args.new_binary).resolve()
OLD = Path(args.old_binary).resolve()
VERSION = subprocess.check_output([str(NEW), '--version'], text=True).strip().split()[-1]


class UpgradeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='cc-install-test-')
        self.home = Path(self.tmp.name)
        self.install = self.home / 'install'
        self.install.mkdir()
        shutil.copy2(OLD, self.install / 'cc-switch-ui')
        (self.install / 'dist').mkdir()
        (self.install / 'dist/index.html').write_text('old frontend')
        self.data = self.home / '.cc-switch'
        self.data.mkdir()
        self.db = self.data / 'cc-switch.db'
        with sqlite3.connect(self.db) as c:
            c.executescript((ROOT / 'cc-switch-lib/tests/fixtures/v0.2.0.sql').read_text())
        self.oauth = b'{"accounts":[],"testMarker":"preserve-oauth"}'
        (self.data / 'codex_oauth_auth.json').write_bytes(self.oauth)
        (self.data / 'skills/legacy').mkdir(parents=True)
        (self.data / 'skills/legacy/SKILL.md').write_text('preserve source')
        (self.home / '.claude').mkdir()
        (self.home / '.claude/settings.json').write_text('{"unrelated":"preserve live"}')
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0))
            self.port = s.getsockname()[1]
        (self.data / 'cli.json').write_text(json.dumps({'host': '127.0.0.1', 'port': self.port, 'proxy_port': 15721}))
        self.mock = self.home / 'mock'
        self.mock.mkdir()
        self.env = os.environ.copy()
        self.env.update(HOME=str(self.home), CC_SWITCH_TEST_HOME=str(self.home),
                        XDG_CONFIG_HOME=str(self.home / 'xdg'), APPDATA=str(self.home),
                        CC_SWITCH_INSTALL_DIR=str(self.install), CC_SWITCH_ADMIN_TOKEN='upgrade-test-token',
                        PATH=str(self.mock) + os.pathsep + self.env['PATH'])
        self.env.pop('CC_SWITCH_UI_DIR', None)
        self.processes = []
        self.package()
        self.mock_curl()

    def tearDown(self):
        pid_file = self.data / 'cc-switch-server.pid'
        if pid_file.exists():
            try:
                pid = int(pid_file.read_text())
                # Isolated PID file belongs only to binaries this test launches.
                if pid > 1:
                    os.kill(pid, signal.SIGTERM)
            except (ValueError, ProcessLookupError):
                pass
        for p in self.processes:
            if p.poll() is None:
                p.terminate()
            p.wait(timeout=10)
        self.tmp.cleanup()

    def package(self, failing=False):
        staging = self.home / 'package/cc-switch-ui'
        staging.mkdir(parents=True, exist_ok=True)
        binary = staging / 'cc-switch-ui'
        if failing:
            binary.write_text(f'''#!/usr/bin/env python3
import os,sys,sqlite3
from pathlib import Path
if '--version' in sys.argv: print('cc-switch-ui {VERSION}'); sys.exit(0)
if 'start' in sys.argv:
    home=Path(os.environ['HOME'])/'.cc-switch'
    with sqlite3.connect(home/'cc-switch.db') as c: c.execute('PRAGMA user_version=99')
    (home/'codex_oauth_auth.json').write_text('damaged by failed startup')
sys.exit(1)
''')
            binary.chmod(0o755)
        else:
            shutil.copy2(NEW, binary)
        (staging / 'dist').mkdir(exist_ok=True)
        (staging / 'dist/index.html').write_text('new frontend')
        self.asset = self.home / 'release.tar.gz'
        with tarfile.open(self.asset, 'w:gz', compresslevel=1) as tar:
            tar.add(staging, arcname='cc-switch-ui')
        self.checksum = self.home / 'release.sha256'
        self.checksum.write_text(hashlib.sha256(self.asset.read_bytes()).hexdigest() + '  release.tar.gz\n')

    def mock_curl(self):
        curl = self.mock / 'curl'
        curl.write_text(f'''#!/usr/bin/env python3
import sys,shutil
args=sys.argv[1:]
url=next(a for a in args if a.startswith('https://'))
if url.endswith('/latest'): print('{{"tag_name":"v{VERSION}"}}')
else:
    source={str(self.asset)!r} if not url.endswith('.sha256') else {str(self.checksum)!r}
    shutil.copyfile(source,args[args.index('-o')+1])
''')
        curl.chmod(0o755)

    def run_installer(self):
        return subprocess.run(['bash', str(ROOT / 'install.sh')], env=self.env,
                              capture_output=True, text=True, timeout=40)

    def start_old(self, options=()):
        log = open(self.home / 'old.log', 'w')
        p = subprocess.Popen([str(self.install / 'cc-switch-ui'), 'start', *options], env=self.env,
                             stdout=log, stderr=log)
        log.close()
        self.processes.append(p)
        self.wait_health()
        return p

    def wait_health(self):
        for _ in range(100):
            try:
                with urllib.request.urlopen(f'http://127.0.0.1:{self.port}/health', timeout=.2) as r:
                    if r.status == 200:
                        return
            except OSError:
                time.sleep(.1)
        self.fail('service did not become healthy')

    def assert_data(self, version):
        with sqlite3.connect(self.db) as c:
            self.assertEqual(c.execute('pragma user_version').fetchone()[0], version)
            self.assertEqual(c.execute("select is_current from providers where id='legacy'").fetchone()[0], 1)
            self.assertEqual(c.execute("select enabled from skills where id='legacy'").fetchone()[0], 0)
            self.assertEqual(c.execute("select name from mcp_servers where id='legacy'").fetchone()[0], 'Legacy MCP')
        self.assertEqual((self.data / 'codex_oauth_auth.json').read_bytes(), self.oauth)
        self.assertEqual((self.data / 'skills/legacy/SKILL.md').read_text(), 'preserve source')
        self.assertEqual((self.home / '.claude/settings.json').read_text(), '{"unrelated":"preserve live"}')

    def test_idle_upgrade_migrates_preserves_data_and_does_not_leave_service_running(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assert_data(4)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'new frontend')
        backups = list((self.data / 'backups/upgrades').glob('*.db'))
        self.assertEqual(len(backups), 1)
        self.assertEqual(backups[0].stat().st_mode & 0o777, 0o600)
        second = self.run_installer()
        self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
        self.assertEqual(len(list((self.data / 'backups/upgrades').glob('*.db'))), 1)
        with self.assertRaises(OSError):
            urllib.request.urlopen(f'http://127.0.0.1:{self.port}/health', timeout=.2)

    def test_running_upgrade_restarts_service_with_old_records(self):
        old = self.start_old()
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIsNotNone(old.poll())
        self.wait_health()
        self.assert_data(4)

    def test_running_upgrade_preserves_explicit_port_and_login_token(self):
        # Stored defaults deliberately differ from the currently running command.
        (self.data / 'cli.json').write_text(json.dumps({'host': '127.0.0.1', 'port': 1, 'proxy_port': 15721}))
        self.start_old(['--host', '127.0.0.1', '--port', str(self.port), '--admin-token', 'original-login-token'])
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.wait_health()
        request = urllib.request.Request(f'http://127.0.0.1:{self.port}/api/managed-deletion', headers={'Authorization': 'Bearer original-login-token'})
        with urllib.request.urlopen(request, timeout=2) as response:
            self.assertEqual(response.status, 200)
        self.assert_data(4)

    def test_failed_running_upgrade_restores_original_program_data_and_old_service(self):
        old_hash = hashlib.sha256((self.install / 'cc-switch-ui').read_bytes()).digest()
        self.start_old()
        self.package(failing=True)
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Previous service and data restored', result.stderr)
        self.assertEqual(hashlib.sha256((self.install / 'cc-switch-ui').read_bytes()).digest(), old_hash)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'old frontend')
        self.wait_health()
        self.assert_data(3)

    def test_checksum_failure_leaves_running_old_service_untouched(self):
        old = self.start_old()
        original = self.db.read_bytes()
        self.checksum.write_text('0' * 64 + '\n')
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(old.poll())
        self.assertEqual(self.db.read_bytes(), original)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'old frontend')

    def test_backup_failure_restarts_old_service_without_replacing_program(self):
        self.start_old()
        real_tar = shutil.which('tar')
        tar = self.mock / 'tar'
        tar.write_text(f'#!/bin/bash\nif [ "$1" = "-czf" ]; then exit 1; fi\nexec {real_tar} "$@"\n')
        tar.chmod(0o755)
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Previous service and data restored', result.stderr)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'old frontend')
        self.wait_health()
        self.assert_data(3)

    def test_failed_idle_upgrade_restores_data_and_keeps_service_stopped(self):
        self.package(failing=True)
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assert_data(3)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'old frontend')
        with self.assertRaises(OSError):
            urllib.request.urlopen(f'http://127.0.0.1:{self.port}/health', timeout=.2)

    def test_missing_checksum_stops_before_changing_old_installation(self):
        self.checksum.unlink()
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Release lacks a checksum', result.stderr)
        self.assert_data(3)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'old frontend')

    def test_fresh_install_is_validated_without_leaving_a_service_running(self):
        shutil.rmtree(self.install)
        shutil.rmtree(self.data)
        # A configured ephemeral port isolates validation from any real service on 5007.
        self.data.mkdir()
        (self.data / 'cli.json').write_text(json.dumps({'host': '127.0.0.1', 'port': self.port, 'proxy_port': 15721}))
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'new frontend')
        self.assertFalse((self.data / 'backups/upgrades').exists())

    def test_unrelated_healthy_listener_does_not_mask_failed_new_server_binding(self):
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'ok')
            def log_message(self, *unused):
                pass
        server = http.server.HTTPServer(('127.0.0.1', self.port), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            original = self.db.read_bytes()
            result = self.run_installer()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('A service is still responding', result.stderr)
            self.assertEqual(self.db.read_bytes(), original)
            self.assertEqual((self.install / 'dist/index.html').read_text(), 'old frontend')
            self.assert_data(3)
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def test_profile_upgrade_lock_blocks_other_installations_without_modification(self):
        lock = self.home / '.cc-switch.upgrade-lock'
        lock.mkdir()
        original = self.db.read_bytes()
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(lock.is_dir())
        self.assertFalse(Path(str(self.install) + '.upgrade-lock').exists())
        self.assertEqual(self.db.read_bytes(), original)
        self.assertEqual((self.install / 'dist/index.html').read_text(), 'old frontend')

    def test_foreign_service_pid_is_not_stopped(self):
        p = subprocess.Popen(['sleep', '60'])
        self.processes.append(p)
        (self.data / 'cc-switch-server.pid').write_text(str(p.pid))
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('not from this installation', result.stderr)
        self.assertIsNone(p.poll())
        self.assert_data(3)


if __name__ == '__main__':
    unittest.main(argv=[__file__] + remaining)
