from pathlib import Path

from benchy.catalog import filter_catalog, load_catalog
from benchy.gold import GoldError, ensure_gold

ROOT = Path(__file__).resolve().parents[1]


def _hexyl():
    return filter_catalog(load_catalog(ROOT / "tasks"), task="hexyl")[0]


def test_ensure_gold_uses_injected_cloner_and_runner(tmp_path):
    task = _hexyl()

    def cloner(t, dest):
        dest.mkdir(parents=True)
        (dest / "target" / "release").mkdir(parents=True)
        (dest / "target" / "release" / "hexyl").write_bytes(b"gold")

    def runner(cmd, cwd):
        assert isinstance(cmd, str)
        return 0

    path = ensure_gold(task, tmp_path / "cache", runner=runner, cloner=cloner)
    assert path.read_bytes() == b"gold"


def test_ensure_gold_raises_when_binary_missing(tmp_path):
    task = _hexyl()

    def cloner(t, dest):
        dest.mkdir(parents=True)

    def runner(cmd, cwd):
        return 0

    try:
        ensure_gold(task, tmp_path / "cache", runner=runner, cloner=cloner)
        raise AssertionError("expected GoldError")
    except GoldError as e:
        assert e.code == "error:gold"
