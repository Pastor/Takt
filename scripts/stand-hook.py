#!/usr/bin/env python3
"""Выкатка стенда по уведомлению GitHub, по опросу и вручную.

Три режима и один носитель шагов:

    stand-hook.py deploy  выкатить ветку: замок, fetch, checkout, stand.sh up
    stand-hook.py serve   слушать уведомления GitHub и выкатывать по ним
    stand-hook.py poll    сверить ветку в origin с выкаченной и выкатить

# Один носитель

Ручная выкатка (`deploy-stand.sh`), уведомление и опрос зовут `deploy`. Шаги,
записанные в двух местах, расходятся молча: ручная выкатка поднимала бы одно,
автоматическая - другое.

Ручная выкатка исполняет носитель из выкатываемой версии (`git show
origin/<ветка>:scripts/stand-hook.py | python3 - deploy`): скрипт, который
`git checkout` переписывает посреди собственного исполнения, исполнял бы
смесь двух версий.

# Замок

На стенде идёт одна выкатка за раз: два `docker compose up --build` разом
спорят за один образ и один контейнер. Замок - `flock` на файле в каталоге
состояния; вторая выкатка ждёт первую.

# Подпись

GitHub подписывает тело HMAC-SHA256 секретом и кладёт подпись в
`X-Hub-Signature-256`. Приёмник без проверки подписи запускал бы выкатку для
любого, кто знает адрес, поэтому без секрета он не стартует, а тело запроса
с негодной подписью не разбирается вовсе. Из тела берутся только ветка и
коммит.

# Настройки (окружение)

    TAKT_HOOK_SECRET   секрет уведомлений (обязателен для serve)
    TAKT_HOOK_BRANCH   ветка выкатки, умолчание v2
    TAKT_HOOK_DIR      каталог клона, умолчание - корень дерева скрипта
    TAKT_HOOK_STATE    каталог состояния, умолчание ~/.local/state/takt-hook
    TAKT_HOOK_LISTEN   адрес приёмника, умолчание 127.0.0.1:8739
    TAKT_HOOK_PREFIX   префикс адресов, умолчание /takt
    TAKT_HOOK_COMMAND  команда подъёма вместо scripts/stand.sh up (проверки)
"""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import hmac
import json
import os
import subprocess
import sys
import threading
import time
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

# Наибольшее тело уведомления: уведомление о пуше несёт список коммитов, и
# мегабайта хватает с запасом, а чтение без предела задало бы расход памяти
# чужой рукой.
BODY_LIMIT = 1024 * 1024


def env(name: str, default: str) -> str:
    return os.environ.get(name, default)


def repo_dir(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit)
    if "TAKT_HOOK_DIR" in os.environ:
        return Path(os.environ["TAKT_HOOK_DIR"])
    # Скрипт, пришедший через stdin, пути у себя не имеет: каталог обязан
    # прийти ключом либо окружением.
    if not globals().get("__file__") or __file__ == "<stdin>":
        sys.exit("stand-hook: каталог клона не задан: --dir либо TAKT_HOOK_DIR")
    return Path(__file__).resolve().parent.parent


def state_dir() -> Path:
    path = Path(env("TAKT_HOOK_STATE", str(Path.home() / ".local/state/takt-hook")))
    path.mkdir(parents=True, exist_ok=True)
    return path


def log(message: str) -> None:
    # Строка журнала - в поток ошибок: его забирает journald.
    print(f"stand-hook: {message}", file=sys.stderr, flush=True)


def read_state() -> dict:
    try:
        return json.loads((state_dir() / "status.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def write_state(state: dict) -> None:
    path = state_dir() / "status.json"
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, ensure_ascii=False, indent=2), encoding="utf-8")
    # Замена целиком: читатель статуса не должен увидеть файл наполовину.
    temporary.replace(path)


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def deploy(repo: Path, branch: str, wanted: str | None = None, force: bool = False) -> bool:
    """Выкатывает ветку под замком стенда; `True` - выкатка удалась.

    `wanted` - коммит, ради которого позвали. Ветка уже выкачена удачно на
    своей вершине - выкатывать нечего, если выкатку не просят явно (`force`):
    ручная выкатка поднимает стек и после правки окружения стенда.
    """
    with open(state_dir() / "deploy.lock", "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            git(repo, "fetch", "--prune", "origin")
            head = git(repo, "rev-parse", f"origin/{branch}")
        except subprocess.CalledProcessError as error:
            log(f"fetch не удался: {error.stderr.strip()}")
            finish(branch, None, "failure", "fetch")
            return False
        if not force and read_state().get("commit") == head and read_state().get("result") == "success":
            log(f"{branch} уже выкачена на {head[:12]}")
            return True
        if wanted and wanted != head:
            log(f"уведомление о {wanted[:12]}, на ветке уже {head[:12]} - выкатываем ветку")
        state = {"branch": branch, "commit": head, "started": int(time.time()), "result": "running"}
        write_state(state)
        log(f"выкатка {branch} на {head[:12]}")
        try:
            git(repo, "checkout", "-B", branch, f"origin/{branch}")
            git(repo, "reset", "--hard", f"origin/{branch}")
        except subprocess.CalledProcessError as error:
            log(f"checkout не удался: {error.stderr.strip()}")
            finish(branch, head, "failure", "checkout")
            return False
        command = env("TAKT_HOOK_COMMAND", "scripts/stand.sh up")
        done = subprocess.run(["bash", "-c", command], cwd=repo)
        if done.returncode != 0:
            log(f"подъём не удался (код {done.returncode}); журнал - выше")
            finish(branch, head, "failure", "up")
            return False
        finish(branch, head, "success", None)
        log(f"выкачено: {head[:12]}")
        return True


def finish(branch: str, commit: str | None, result: str, step: str | None) -> None:
    state = read_state()
    state.update({"branch": branch, "result": result, "finished": int(time.time())})
    if commit:
        state["commit"] = commit
    if step:
        state["step"] = step
    else:
        state.pop("step", None)
    write_state(state)


def signature_ok(secret: bytes, body: bytes, header: str | None) -> bool:
    if not header or not header.startswith("sha256="):
        return False
    expected = "sha256=" + hmac.new(secret, body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(expected, header)


class Queue:
    """Ожидающие выкатки: хранится один, последний коммит."""

    def __init__(self) -> None:
        self.pending: str | None = None
        self.wake = threading.Condition()

    def put(self, commit: str) -> None:
        with self.wake:
            self.pending = commit
            self.wake.notify()

    def take(self) -> str:
        with self.wake:
            while self.pending is None:
                self.wake.wait()
            commit, self.pending = self.pending, None
            return commit


def worker(queue: Queue, repo: Path, branch: str) -> None:
    while True:
        commit = queue.take()
        try:
            deploy(repo, branch, commit)
        except Exception as error:  # noqa: BLE001 - рабочий поток не должен умирать
            log(f"выкатка оборвалась: {error}")


def handler(secret: bytes, branch: str, prefix: str, queue: Queue):
    hook = f"{prefix}/hooks/github"
    status = f"{prefix}/hooks/status"

    class Handler(BaseHTTPRequestHandler):
        def reply(self, code: HTTPStatus, payload: dict) -> None:
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(code)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_args) -> None:
            # Строка на каждый запрос забила бы журнал; значимое пишется само.
            pass

        def do_GET(self) -> None:
            if self.path != status:
                return self.reply(HTTPStatus.NOT_FOUND, {"error": "нет такого адреса"})
            state = read_state()
            shown = {key: state[key] for key in ("branch", "commit", "result", "started", "finished", "step") if key in state}
            return self.reply(HTTPStatus.OK, shown)

        def do_POST(self) -> None:
            if self.path != hook:
                return self.reply(HTTPStatus.NOT_FOUND, {"error": "нет такого адреса"})
            try:
                length = int(self.headers.get("Content-Length", ""))
            except ValueError:
                return self.reply(HTTPStatus.LENGTH_REQUIRED, {"error": "нужна длина тела"})
            if length < 0 or length > BODY_LIMIT:
                return self.reply(HTTPStatus.REQUEST_ENTITY_TOO_LARGE, {"error": f"тело больше {BODY_LIMIT} байт"})
            body = self.rfile.read(length)
            if not signature_ok(secret, body, self.headers.get("X-Hub-Signature-256")):
                log(f"негодная подпись с {self.headers.get('X-Forwarded-For', self.client_address[0])}")
                return self.reply(HTTPStatus.UNAUTHORIZED, {"error": "подпись не сошлась"})
            event = self.headers.get("X-GitHub-Event", "")
            if event == "ping":
                return self.reply(HTTPStatus.OK, {"result": "pong"})
            if event != "push":
                return self.reply(HTTPStatus.ACCEPTED, {"result": "пропущено", "reason": f"событие {event}"})
            try:
                payload = json.loads(body)
                ref = str(payload["ref"])
                commit = str(payload["after"])
            except (ValueError, KeyError, TypeError):
                return self.reply(HTTPStatus.BAD_REQUEST, {"error": "нет ref либо after"})
            if ref != f"refs/heads/{branch}":
                return self.reply(HTTPStatus.ACCEPTED, {"result": "пропущено", "reason": f"ветка {ref}"})
            if not (len(commit) == 40 and all(c in "0123456789abcdef" for c in commit)) or set(commit) == {"0"}:
                return self.reply(HTTPStatus.ACCEPTED, {"result": "пропущено", "reason": "нет коммита"})
            queue.put(commit)
            log(f"принято уведомление: {branch} на {commit[:12]}")
            return self.reply(HTTPStatus.ACCEPTED, {"result": "принято", "commit": commit})

    return Handler


def serve(repo: Path, branch: str) -> None:
    secret = os.environ.get("TAKT_HOOK_SECRET", "")
    if not secret:
        sys.exit("stand-hook: TAKT_HOOK_SECRET не задан: без подписи выкатку запускал бы кто угодно")
    host, _, port = env("TAKT_HOOK_LISTEN", "127.0.0.1:8739").rpartition(":")
    prefix = env("TAKT_HOOK_PREFIX", "/takt").rstrip("/")
    queue = Queue()
    threading.Thread(target=worker, args=(queue, repo, branch), daemon=True).start()
    server = ThreadingHTTPServer((host, int(port)), handler(secret.encode(), branch, prefix, queue))
    log(f"слушаю {host}:{server.server_address[1]}, ветка {branch}, адрес {prefix}/hooks/github")
    server.serve_forever()


def poll(repo: Path, branch: str) -> bool:
    try:
        line = git(repo, "ls-remote", "origin", f"refs/heads/{branch}")
    except subprocess.CalledProcessError as error:
        log(f"ls-remote не удался: {error.stderr.strip()}")
        return False
    remote = line.split()[0] if line else ""
    state = read_state()
    if remote and state.get("commit") == remote and state.get("result") == "success":
        return True
    log(f"опрос: в origin {remote[:12] or 'пусто'}, выкачено {str(state.get('commit', ''))[:12] or 'ничего'}")
    return deploy(repo, branch, remote or None)


def main() -> None:
    parser = argparse.ArgumentParser(description="Выкатка стенда Takt")
    parser.add_argument("mode", choices=["deploy", "serve", "poll"])
    parser.add_argument("--dir", help="каталог клона на стенде")
    parser.add_argument("--branch", default=env("TAKT_HOOK_BRANCH", "v2"))
    parser.add_argument("--force", action="store_true", help="выкатить и выкаченное")
    args = parser.parse_args()
    repo = repo_dir(args.dir)
    if args.mode == "deploy":
        sys.exit(0 if deploy(repo, args.branch, force=args.force) else 1)
    if args.mode == "poll":
        sys.exit(0 if poll(repo, args.branch) else 1)
    serve(repo, args.branch)


if __name__ == "__main__":
    main()
