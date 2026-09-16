#!/usr/bin/env python3
"""Случаи проверки выкатки стенда (`scripts/stand-hook.py`).

Зовётся из `scripts/test-stand-hook.sh`: там же описаны условия H1…H12 и
контроль. Путь проверяемого приёмника и рабочий каталог приходят окружением
(`HOOK`, `WORK`), чтобы контроль мог подставить приёмник с изъятой проверкой.
"""
import hashlib, hmac, json, os, socket, subprocess, sys, time, urllib.request, urllib.error
from pathlib import Path

work = Path(os.environ["WORK"])
hook = os.environ["HOOK"]

def run(*args, cwd=None, check=True, env=None):
    return subprocess.run(list(args), cwd=cwd, check=check, capture_output=True, text=True, env=env)

SERVERS = []

def fail(message):
    for process in SERVERS:
        process.kill()
    print(f"  ПРОВАЛ: {message}")
    sys.exit(1)

origin = work / "origin.git"
author = work / "author"
stand = work / "stand"
state = work / "state"
ups = work / "ups.log"
run("git", "init", "--bare", "-q", "-b", "v2", str(origin))
run("git", "clone", "-q", str(origin), str(author))
ident = ["-c", "user.name=проба", "-c", "user.email=probe@example.invalid"]

def commit(text):
    (author / "f.txt").write_text(text)
    run("git", "-C", str(author), "add", "f.txt")
    run("git", *ident, "-C", str(author), "commit", "-q", "-m", text)
    run("git", "-C", str(author), "push", "-q", "origin", "HEAD:v2")
    run("git", "-C", str(author), "push", "-q", "origin", "HEAD:other")
    return run("git", "-C", str(author), "rev-parse", "HEAD").stdout.strip()

c1 = commit("один")
run("git", "clone", "-q", "-b", "v2", str(origin), str(stand))

base = dict(os.environ)
base.update({
    "TAKT_HOOK_DIR": str(stand),
    "TAKT_HOOK_STATE": str(state),
    "TAKT_HOOK_BRANCH": "v2",
    "TAKT_HOOK_PREFIX": "/takt",
    # Подъём записывает коммит, начало и конец: по ним видно и что выкачено,
    # и что выкатки не пересекались.
    "TAKT_HOOK_COMMAND": f'echo "$(git rev-parse HEAD) $(date +%s.%N 2>/dev/null || date +%s) start" >> {ups}; sleep 1; echo "$(git rev-parse HEAD) $(python3 -c "import time; print(time.time())") end" >> {ups}',
})

def ups_commits():
    if not ups.exists():
        return []
    return [line.split()[0] for line in ups.read_text().splitlines() if line.endswith("end")]

def wait(predicate, what, seconds=20):
    deadline = time.time() + seconds
    while time.time() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    fail(what)

# -- H1 --------------------------------------------------------------------
env = dict(base)
env.pop("TAKT_HOOK_SECRET", None)
done = run("python3", hook, "serve", check=False, env=env)
if done.returncode == 0 or "TAKT_HOOK_SECRET" not in done.stderr:
    fail(f"H1 приёмник без секрета стартовал: {done.returncode} {done.stderr}")
print("  + H1 без секрета приёмник не стартует")

secret = "секрет-проверки".encode()
sock = socket.socket(); sock.bind(("127.0.0.1", 0)); port = sock.getsockname()[1]; sock.close()
env = dict(base)
env.update({"TAKT_HOOK_SECRET": secret.decode(), "TAKT_HOOK_LISTEN": f"127.0.0.1:{port}"})
server = subprocess.Popen(["python3", hook, "serve"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
SERVERS.append(server)
url = f"http://127.0.0.1:{port}/takt/hooks"

def call(path, body=b"", headers=None, method="POST"):
    request = urllib.request.Request(url + path, data=body if method == "POST" else None, method=method, headers=headers or {})
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            return response.status, json.loads(response.read() or b"null")
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read() or b"null")

def signed(payload, event="push", key=secret):
    body = json.dumps(payload).encode()
    digest = "sha256=" + hmac.new(key, body, hashlib.sha256).hexdigest()
    return body, {"X-Hub-Signature-256": digest, "X-GitHub-Event": event, "Content-Type": "application/json"}

wait(lambda: socket.socket().connect_ex(("127.0.0.1", port)) == 0, "приёмник не поднялся")

def push(commit_id, ref="refs/heads/v2", event="push", key=secret):
    body, headers = signed({"ref": ref, "after": commit_id}, event, key)
    return call("/github", body, headers)

# -- H2 --------------------------------------------------------------------
status, _ = push(c1, key="чужой-секрет".encode())
if status != 401:
    fail(f"H2 чужой секрет: {status}")
body, headers = signed({"ref": "refs/heads/v2", "after": c1})
headers.pop("X-Hub-Signature-256")
status, _ = call("/github", body, headers)
if status != 401:
    fail(f"H2 без подписи: {status}")
time.sleep(1.5)
if ups_commits():
    fail("H2 выкатка запущена без годной подписи")
print("  + H2 негодная и отсутствующая подпись - 401, выкатки нет")

# -- H3 --------------------------------------------------------------------
status, answer = push(c1, ref="refs/heads/other")
if status != 202 or answer.get("result") != "пропущено":
    fail(f"H3 чужая ветка: {status} {answer}")
status, answer = push(c1, event="issues")
if status != 202 or answer.get("result") != "пропущено":
    fail(f"H3 чужое событие: {status} {answer}")
status, answer = push(c1, event="ping")
if status != 200:
    fail(f"H3 ping: {status} {answer}")
time.sleep(1.5)
if ups_commits():
    fail("H3 выкатка запущена по чужой ветке либо событию")
print("  + H3 чужая ветка и чужое событие пропущены")

# -- H4 --------------------------------------------------------------------
started = time.time()
status, answer = push(c1)
if status != 202 or answer.get("result") != "принято":
    fail(f"H4 годное уведомление: {status} {answer}")
if time.time() - started > 0.9:
    fail("H4 ответ ждал выкатки")
wait(lambda: ups_commits() == [c1], f"H4 выкатка {c1[:12]} не прошла: {ups_commits()}")
print("  + H4 годное уведомление выкатывает коммит, ответ не ждёт выкатки")

# -- H5 --------------------------------------------------------------------
wait(lambda: json.loads((state / "status.json").read_text()).get("result") == "success", "H5 статус не success")
push(c1)
time.sleep(2)
if ups_commits() != [c1]:
    fail(f"H5 повтор выкатил снова: {ups_commits()}")
print("  + H5 повтор выкаченного коммита выкатки не запускает")

# -- H6 --------------------------------------------------------------------
c2 = commit("два")
push(c2)
time.sleep(0.3)
c3 = commit("три")
push(c3)
push(c3)
push(c3)
wait(lambda: ups_commits()[-1:] == [c3], f"H6 последний коммит не выкачен: {ups_commits()}")
time.sleep(2)
tail = ups_commits()[1:]
if tail not in ([c2, c3], [c3]):
    fail(f"H6 выкатки не схлопнулись: {[c[:7] for c in tail]}")
print(f"  + H6 уведомления во время выкатки схлопнулись: выкаток {len(tail)} на четыре уведомления")

# -- H7 --------------------------------------------------------------------
status, shown = call("/status", method="GET")
if status != 200 or shown.get("commit") != c3 or shown.get("result") != "success":
    fail(f"H7 статус: {status} {shown}")
if set(shown) - {"branch", "commit", "result", "started", "finished", "step"}:
    fail(f"H7 статус несёт лишнее: {sorted(shown)}")
status, _ = call("/nothing", method="GET")
if status != 404:
    fail(f"H7 чужой адрес: {status}")
print("  + H7 статус отдаёт коммит и исход")

# -- H11 -------------------------------------------------------------------
# Заголовок называет длину больше предела, тело не шлётся: отказ обязан прийти
# до чтения тела.
import http.client
connection = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
connection.putrequest("POST", "/takt/hooks/github")
connection.putheader("X-GitHub-Event", "push")
connection.putheader("Content-Length", str(1024 * 1024 + 1))
connection.endheaders()
status = connection.getresponse().status
connection.close()
if status != 413:
    fail(f"H11 большое тело: {status}")
print("  + H11 тело больше предела - 413")

server.terminate(); server.wait(5)

# -- H8 --------------------------------------------------------------------
before = len(ups_commits())
done = run("python3", hook, "poll", check=False, env=base)
if done.returncode != 0 or len(ups_commits()) != before:
    fail(f"H8 опрос без расхождения выкатил: {done.returncode} {done.stderr}")
c4 = commit("четыре")
done = run("python3", hook, "poll", check=False, env=base)
if done.returncode != 0 or ups_commits()[-1] != c4 or len(ups_commits()) != before + 1:
    fail(f"H8 опрос не выкатил расхождение: {done.stderr}")
print("  + H8 опрос выкатывает только при расхождении")

# -- H9 --------------------------------------------------------------------
c5 = commit("пять")
broken = dict(base); broken["TAKT_HOOK_COMMAND"] = "exit 3"
done = run("python3", hook, "deploy", check=False, env=broken)
saved = json.loads((state / "status.json").read_text())
if done.returncode == 0 or saved.get("result") != "failure" or saved.get("step") != "up" or saved.get("commit") != c5:
    fail(f"H9 провал не записан: {done.returncode} {saved}")
done = run("python3", hook, "poll", check=False, env=base)
saved = json.loads((state / "status.json").read_text())
if done.returncode != 0 or ups_commits()[-1] != c5 or saved.get("result") != "success" or "step" in saved:
    fail(f"H9 выкатка после провала не прошла: {saved}")
print("  + H9 провал подъёма записан, следующая выкатка его чинит")

# -- H10 -------------------------------------------------------------------
ups.write_text("")
first = subprocess.Popen(["python3", hook, "deploy", "--force"], env=base, stderr=subprocess.DEVNULL)
second = subprocess.Popen(["python3", hook, "deploy", "--force"], env=base, stderr=subprocess.DEVNULL)
if first.wait(30) != 0 or second.wait(30) != 0:
    fail("H10 ручные выкатки отказали")
marks = [line.split() for line in ups.read_text().splitlines()]
if [mark[2] for mark in marks] != ["start", "end", "start", "end"]:
    fail(f"H10 выкатки пересеклись: {marks}")
print("  + H10 две ручные выкатки разом идут по очереди")
