import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


binary = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="xdd-smoke-") as temporary:
    home = Path(temporary)
    config = home / "config" / "xdd" / "config.toml"
    projects = home / "projects"
    target = projects / "hello world"
    target.mkdir(parents=True)
    config.parent.mkdir(parents=True)
    config.write_text(
        f"[roots]\nprojects = {json.dumps(str(projects))}\n", encoding="utf-8"
    )
    environment = {
        **os.environ,
        "HOME": str(home),
        "XDG_CONFIG_HOME": str(home / "config"),
        "XDG_DATA_HOME": str(home / "data"),
    }

    def run(*arguments, success=True):
        result = subprocess.run(
            [str(binary), *arguments],
            env=environment,
            cwd=home,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if (result.returncode == 0) != success:
            raise AssertionError(f"{arguments}: {result.returncode}: {result.stderr}")
        return result.stdout.rstrip("\n")

    run("--help")
    assert run("config", "path") == str(config)
    assert run("roots", "list") == f"projects -> {projects}"
    url = run("link", str(target))
    assert url == "xdd://projects:hello%20world"
    assert run("resolve", url) == str(target)
    assert run("link", "projects/hello world") == url
    assert run("link", str(target), "--format", "markdown", "--label", "Example") == (
        f"[Example]({url})"
    )
    run("resolve", "xdd://projects:../outside", success=False)
    run("link", str(home), success=False)

print("Smoke tests passed")
