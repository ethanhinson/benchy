from pathlib import Path

import pytest

from benchy.catalog import CatalogError, filter_catalog, load_catalog

ROOT = Path(__file__).resolve().parents[1]


def test_first_slice_instance_ids():
    tasks = load_catalog(ROOT / "tasks")
    first = filter_catalog(tasks, slice="first")
    ids = {t.instance_id for t in first}
    assert ids == {
        "sharkdp__hexyl.2e26437",
        "riquito__tuc.16fb471",
        "oppiliappan__eva.41ae245",
    }


def test_nested_probes_are_not_tasks():
    slugs = {t.slug for t in load_catalog(ROOT / "tasks")}
    assert slugs == {"hexyl", "tuc", "eva"}
    assert (ROOT / "tasks" / "hexyl" / "probes.yaml").is_file()


def test_missing_field_raises(tmp_path):
    (tmp_path / "bad.yaml").write_text("slug: bad\ninstance_id: x\n")
    with pytest.raises(CatalogError):
        load_catalog(tmp_path)


def test_non_mapping_top_level_raises(tmp_path):
    (tmp_path / "list.yaml").write_text("- not: a task\n")
    with pytest.raises(CatalogError):
        load_catalog(tmp_path)


def test_filter_one_task():
    tasks = load_catalog(ROOT / "tasks")
    only = filter_catalog(tasks, slice="first", task="hexyl")
    assert [t.slug for t in only] == ["hexyl"]
