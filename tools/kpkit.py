#!/usr/bin/env python3
"""RustWay docs→KP 转换流水线工具。

职责：
- 由源文件相对路径确定性生成知识点 id 与输出文件名（base36(md5)，无中央计数器）；
- 维护覆盖清单 tools/docs2kps-manifest.json：每个 docs 源文件 → 输出 JSON + KP ids，
  作为「docs 内所有文件全部生成」的可审计账本；
- 基础校验：JSON 可解析、id 唯一、id 形状合法、来源文件存在。

子命令：
  id-for <prefix> <src-relpath>          打印 id 与建议输出文件名
  register <output.json> <src...>        登记一个已完成文件（校验后写入清单）
  coverage [repo-dir]                    打印覆盖率与未覆盖源文件列表
"""
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent          # rustway 仓库根
DOCS = ROOT.parent / "docs"                            # docs 资料库根
MANIFEST = ROOT / "tools" / "docs2kps-manifest.json"
ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")

PREFIXES = {
    "cn": "CS-Notes-master",
    "jg": "JavaGuide-main",
    "sg": "CSStudyGuide-main",
    "sl": "cs-self-learning-master",
    "fb": "free-programming-books-main",
}


def b36(n: int) -> str:
    digits = "0123456789abcdefghijklmnopqrstuvwxyz"
    out = ""
    while n:
        out = digits[n % 36] + out
        n //= 36
    return out or "0"


def digest(relpath: str) -> str:
    return b36(int(hashlib.sha256(relpath.encode()).hexdigest(), 16)).zfill(11)


def id_for(prefix: str, src_relpath: str, section: int | None = None) -> str:
    kp_id = f"{prefix}-{digest(src_relpath)[:8]}"
    if section:
        kp_id += f"-{section}"
    if not ID_RE.match(kp_id) or len(kp_id) > 32:
        raise ValueError(f"非法 id: {kp_id}")
    return kp_id


def out_name(prefix: str, src_relpath: str) -> str:
    return f"{prefix}-{digest(src_relpath)[:6]}.json"


def load_manifest() -> dict:
    if MANIFEST.exists():
        return json.loads(MANIFEST.read_text())
    return {"version": 1, "entries": []}


def save_manifest(m: dict) -> None:
    MANIFEST.write_text(json.dumps(m, ensure_ascii=False, indent=1) + "\n")


def all_doc_md(repo_dir: str):
    return sorted(DOCS.joinpath(repo_dir).rglob("*.md"))


def cmd_id_for(prefix: str, rel: str, section=None) -> None:
    print(id_for(prefix, rel, section), out_name(prefix, rel), sep="\t")


def cmd_register(output: str, srcs: list[str]) -> None:
    m = load_manifest()
    body = json.loads((ROOT / output).read_text())
    kp_ids = [k["id"] for k in body.get("kps", [])]
    if not kp_ids:
        sys.exit(f"错误：{output} 没有 kps")
    for kp_id in kp_ids:
        if not ID_RE.match(kp_id):
            sys.exit(f"错误：非法 id {kp_id}")
    known_ids = {i for e in m["entries"] for i in e["kp_ids"]}
    dup = known_ids & set(kp_ids)
    if dup:
        sys.exit(f"错误：id 重复登记: {dup}")
    rel_srcs = []
    for s in srcs:
        p = DOCS / s
        if not p.exists():
            sys.exit(f"错误：源文件不存在: {s}")
        rel_srcs.append(
            {"path": s, "sha256": hashlib.sha256(p.read_bytes()).hexdigest(), "bytes": p.stat().st_size}
        )
    m["entries"].append({"output": output, "kp_ids": kp_ids, "sources": rel_srcs})
    m["entries"].sort(key=lambda e: e["output"])
    save_manifest(m)
    print(f"已登记 {output}: {len(kp_ids)} 个知识点 ← {len(rel_srcs)} 个源文件")


def cmd_coverage(prefix: str | None) -> None:
    m = load_manifest()
    covered = {s["path"] for e in m["entries"] for s in e["sources"]}
    total = 0
    missing = []
    repos = [PREFIXES[prefix]] if prefix else PREFIXES.values()
    for repo_dir in repos:
        for p in all_doc_md(repo_dir):
            rel = str(p.relative_to(DOCS))
            total += 1
            if rel not in covered:
                missing.append(rel)
    print(f"覆盖 {total - len(missing)}/{total} 个 docs 源文件")
    for rel in missing:
        print("未覆盖:", rel)
    if missing:
        sys.exit(1)


def main() -> None:
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "id-for":
        sec = int(sys.argv[4]) if len(sys.argv) > 4 else None
        cmd_id_for(sys.argv[2], sys.argv[3], sec)
    elif cmd == "register":
        cmd_register(sys.argv[2], sys.argv[3:])
    elif cmd == "coverage":
        cmd_coverage(sys.argv[2] if len(sys.argv) > 2 else None)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
