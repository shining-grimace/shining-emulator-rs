#!/usr/bin/env python3
"""Prepare, run, export and verify the pinned ROM catalogs."""
import argparse
import functools
import hashlib
import html
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
import urllib.request
import zipfile

REPO = Path(__file__).resolve().parent.parent
CATALOG = REPO / "test-roms/catalog.json"
UPLOAD = REPO / "target/rom-catalog-upload/home-of-8-bit/rom-catalog"


def public_case(case, in_app=False):
    """Publish only useful fields; version 1 defines breakpoint/12s defaults."""
    result = {key: value for key, value in case.items() if key != "in_app"}
    if in_app:
        for key in ("protocol", "seconds", "skip", "screen", "screen_sha256"):
            result.pop(key, None)
    return result


def digest(data):
    return hashlib.sha256(data).hexdigest()


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "shining-emulator-rs"})
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def safe_path(path):
    if not path or Path(path).is_absolute() or ".." in Path(path).parts or ":" in path or "\\" in path:
        raise ValueError("Unsafe fixture path: " + path)
    return path


def site_document(base, reference):
    data = fetch(base + safe_path(reference["path"]))
    if digest(data) != reference["sha256"]:
        raise ValueError("Site document checksum mismatch: " + reference["path"])
    return data


def prepare_site(root, catalog, base):
    config = json.loads((REPO / "test-roms/site-config.json").read_text())
    expected = json.loads((REPO / "test-roms/site-directory.json").read_text())
    directory = json.loads(fetch(base + "directory.json"))
    if directory["schema_version"] != 1:
        raise ValueError("Unsupported website directory schema")
    releases = [entry for entry in directory["releases"] if entry["id"] == config["release"]]
    pinned = next(entry for entry in expected["releases"] if entry["id"] == config["release"])
    if len(releases) != 1 or releases[0] != pinned:
        raise ValueError("Website release differs from the checked-in directory")
    release = releases[0]
    public = json.loads(site_document(base, release["catalogs"]["headless"]))
    permitted = [case for case in catalog["cases"] if not case["path"].startswith("blargg/")]
    if public["cases"] != [public_case(case) for case in permitted] or public["schema_version"] != 1 or public["release"] != config["release"]:
        raise ValueError("Website headless selection differs from the pinned fixtures")
    version = "v1/" + config["release"]
    for case in {case["path"]: case for case in permitted}.values():
        name = safe_path(case["path"])
        target = root / name
        if target.is_file() and digest(target.read_bytes()) == case["sha256"]:
            continue
        data = fetch(base + version + "/roms/" + name)
        if digest(data) != case["sha256"]:
            raise ValueError("Website ROM checksum mismatch: " + name)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    for notice in release["licenses"]:
        data = site_document(base, notice)
        target = root / "licenses" / Path(notice["path"]).name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)


def prepare_upstream(root, catalog, blargg_only=False):
    archive = root / "collection-v7.0.zip"
    if not archive.exists() or digest(archive.read_bytes()) != catalog["archive_sha256"]:
        print("Downloading " + catalog["collection"], flush=True)
        data = fetch(catalog["url"])
        if digest(data) != catalog["archive_sha256"]:
            raise ValueError("Downloaded test collection checksum mismatch")
        archive.write_bytes(data)
    cases = [case for case in catalog["cases"] if not blargg_only or case["path"].startswith("blargg/")]
    roms = {case["path"]: case for case in cases}
    screens = {case["screen"]: case["screen_sha256"] for case in cases if "screen" in case}
    with zipfile.ZipFile(archive) as files:
        for name in sorted(roms.keys() | screens.keys()):
            safe_path(name)
            data = files.read(name)
            if name in roms and digest(data) != roms[name]["sha256"]:
                raise ValueError("ROM checksum mismatch: " + name)
            if name in screens and digest(data) != screens[name]:
                raise ValueError("Screen checksum mismatch: " + name)
            target = root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    if not blargg_only:
        shutil.copytree(REPO / "test-roms/licenses", root / "licenses", dirs_exist_ok=True)


def verify(root, catalog):
    for case in catalog["cases"]:
        name = safe_path(case["path"])
        path = root / name
        if not path.is_file() or digest(path.read_bytes()) != case["sha256"]:
            raise ValueError("Missing or corrupt ROM; prepare online first: " + name)
        if "screen" in case:
            screen = root / safe_path(case["screen"])
            if not screen.is_file() or digest(screen.read_bytes()) != case["screen_sha256"]:
                raise ValueError("Missing or corrupt golden screen: " + case["screen"])


def prepare(offline=False, source="site", site_url=None):
    catalog = json.loads(CATALOG.read_text())
    root = Path(os.environ.get("SHINING_TEST_ROM_ROOT", REPO / ".test-roms/validation"))
    root.mkdir(parents=True, exist_ok=True)
    if not offline:
        if source == "site":
            config = json.loads((REPO / "test-roms/site-config.json").read_text())
            base = (site_url or config["base_url"]).rstrip("/") + "/"
            prepare_site(root, catalog, base)
        prepare_upstream(root, catalog, blargg_only=source == "site")
    verify(root, catalog)
    print(f"Prepared {len({case['path'] for case in catalog['cases']})} ROMs "
          f"for {len(catalog['cases'])} hardware cases at {root}")


def encoded(value):
    return (json.dumps(value, indent=2) + "\n").encode()


def reference(path, data):
    return {"path": path, "sha256": digest(data)}


def export(output, offline=False):
    prepare(offline=offline, source="upstream")
    root = Path(os.environ.get("SHINING_TEST_ROM_ROOT", REPO / ".test-roms/validation"))
    source = json.loads((REPO / "test-roms/catalog.json").read_text())
    config = json.loads((REPO / "test-roms/site-config.json").read_text())
    version = "v1/" + config["release"]
    cases = [case for case in source["cases"] if not case["path"].startswith("blargg/")]
    app_cases = [case for case in cases if case.get("in_app", False)]
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    # The real directory file identifies an existing export; no marker file.
    if any(output.iterdir()) and not (output / "directory.json").is_file():
        raise ValueError("Export destination must be empty or contain a catalog directory.json")
    previous = json.loads((output / "directory.json").read_text())["releases"] if (output / "directory.json").exists() else []

    def write(path, data):
        target = output / path
        if path.startswith("v1/") and target.exists() and target.read_bytes() != data:
            raise ValueError("Versioned files are immutable; choose a new release ID: " + path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        return reference(path, data)

    licenses = []
    for path in sorted((REPO / "test-roms/licenses").glob("*.txt")):
        licenses.append(write(f"{version}/licenses/{path.name}", path.read_bytes()))
    catalogs = {}
    for name, selection in [("in_app", app_cases), ("headless", cases)]:
        data = encoded({"schema_version": 1, "release": config["release"],
                        "collection": source["collection"], "cases": [public_case(case, in_app=name == "in_app") for case in selection]})
        catalogs[name] = write(f"{version}/{name.replace('_', '-')}.json", data)
        (REPO / f"test-roms/site-{name.replace('_', '-')}.json").write_bytes(data)
    for name in sorted({case["path"] for case in cases}):
        write(f"{version}/roms/{name}", (root / name).read_bytes())
    release = {"id": config["release"], "collection": source["collection"], "catalogs": catalogs, "licenses": licenses}
    directory = {"schema_version": 1, "releases": [entry for entry in previous if entry["id"] != config["release"]] + [release]}
    data = encoded(directory)
    write("directory.json", data)
    (REPO / "test-roms/site-directory.json").write_bytes(data)
    links = [f'<li><a href="{item["path"]}">{html.escape(Path(item["path"]).name)}</a></li>' for item in licenses]
    rows = []
    for case in app_cases:
        rows.append(f'<tr><td>{html.escape(case["name"])}</td><td>{html.escape(case["license"])}</td>'
                    f'<td><a href="{version}/roms/{case["path"]}">ROM</a></td></tr>')
    page = ('<!doctype html><html lang="en"><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width"><title>Game Boy validation ROMs</title>'
            '<h1>Game Boy validation ROMs</h1>'
            f'<p>Release {html.escape(config["release"])}. Sources: Mooneye and SameSuite.</p>'
            '<p><a href="directory.json">Machine-readable directory</a> · '
            f'<a href="{catalogs["in_app"]["path"]}">In-app catalog</a> · '
            f'<a href="{catalogs["headless"]["path"]}">Headless catalog</a></p>'
            '<h2>Sources and credits</h2><ul>'
            '<li>Mooneye Test Suite by Joonas Javanainen and contributors: '
            '<a href="https://github.com/Gekkio/mooneye-test-suite">original source</a> (MIT).</li>'
            '<li>SameSuite by Lior Halphon and contributors: '
            '<a href="https://github.com/LIJI32/SameSuite">original source</a> (X11).</li>'
            '<li>Compiled collection by Christoph Sprenger: '
            '<a href="https://github.com/c-sp/game-boy-test-roms/releases/tag/v7.0">upstream v7.0 release</a> '
            '(collection MIT notice; individual suites retain their own licenses).</li></ul>'
            '<h2>Licenses</h2><ul>' + ''.join(links) + '</ul>'
            '<h2>In-app diagnostics</h2><table><tr><th>Diagnostic</th><th>License</th><th>Download</th></tr>'
            + ''.join(rows) + '</table><p>Each diagnostic requires its named hardware target. '
            'The headless catalog lists additional files and unsupported hardware cases.</p></html>\n')
    write("index.html", page.encode())
    write("README.txt", (f"Upload this directory's contents to {config['base_url']}\n"
          "Keep the v1 directory structure unchanged. Serve .json as application/json,\n"
          ".gb as application/octet-stream, and .txt as text/plain. No server-side code.\n"
          "The root directory may be revalidated; versioned files must remain immutable.\n"
          "Blargg is intentionally excluded. Preserve all license notices.\n").encode())
    print(f"Upload-ready tree: {output}")
    print(f"{len(app_cases)} app entries, {len(cases)} headless cases, "
          f"{len({c['path'] for c in cases})} distinct ROMs; no Blargg")


class CatalogTests(unittest.TestCase):
    def test_upload_inventory_checksums_and_no_blargg(self):
        catalog = json.loads((REPO / "test-roms/catalog.json").read_text())
        expected = {"v1/validation-v7.0-1/roms/" + c["path"]: c for c in catalog["cases"] if not c["path"].startswith("blargg/")}
        actual = {p.relative_to(UPLOAD).as_posix() for p in UPLOAD.rglob("*.gb")}
        self.assertEqual(set(expected), actual)
        self.assertEqual(len(actual), 181)
        self.assertFalse(any("blargg" in path for path in actual))
        for path, case in expected.items():
            self.assertEqual(digest((UPLOAD / path).read_bytes()), case["sha256"])
        self.assertFalse((UPLOAD / ".shining-rom-catalog").exists())
        self.assertFalse((UPLOAD / "SHA256SUMS").exists())
        page = (UPLOAD / "index.html").read_text()
        for credit in ("Joonas Javanainen", "Lior Halphon", "Christoph Sprenger",
                       "github.com/Gekkio/mooneye-test-suite", "github.com/LIJI32/SameSuite",
                       "github.com/c-sp/game-boy-test-roms/releases/tag/v7.0",
                       "Collection-MIT.txt", "Mooneye-MIT.txt", "SameSuite-X11.txt"):
            self.assertIn(credit, page)
        directory = json.loads((UPLOAD / "directory.json").read_text())
        for release in directory["releases"]:
            for ref in list(release["catalogs"].values()) + release["licenses"]:
                self.assertEqual(set(ref), {"path", "sha256"})
                self.assertEqual(digest((UPLOAD / ref["path"]).read_bytes()), ref["sha256"])
        for name in ["directory", "in-app", "headless"]:
            path = "directory.json" if name == "directory" else f"v1/validation-v7.0-1/{name}.json"
            self.assertEqual((UPLOAD / path).read_bytes(), (REPO / f"test-roms/site-{name}.json").read_bytes())

    def test_http_import_preserves_all_pinned_fixtures_and_rejects_bad_downloads(self):
        requests = []
        corrupt = []

        class Handler(http.server.SimpleHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                requests.append(self.path)
                if self.path in corrupt:
                    self.send_response(200)
                    self.end_headers()
                    self.wfile.write(b"corrupt ROM")
                else:
                    super().do_GET()

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(UPLOAD)))
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        base = f"http://127.0.0.1:{server.server_port}/"
        catalog = json.loads((REPO / "test-roms/catalog.json").read_text())
        try:
            with tempfile.TemporaryDirectory(prefix="shining-site-import-") as temporary:
                root = Path(temporary)
                # Blargg must come from the developer cache, never this website.
                source = Path(os.environ.get("SHINING_TEST_ROM_ROOT", REPO / ".test-roms/validation"))
                shutil.copytree(source / "blargg", root / "blargg")
                prepare_site(root, catalog, base)
                verify(root, catalog)
                self.assertFalse(any("blargg" in path for path in requests))
                name = "mooneye-test-suite/acceptance/div_timing.gb"
                (root / name).unlink()
                corrupt.append("/v1/validation-v7.0-1/roms/" + name)
                with self.assertRaisesRegex(ValueError, "ROM checksum mismatch"):
                    prepare_site(root, catalog, base)
                with self.assertRaisesRegex(ValueError, "Missing or corrupt ROM"):
                    verify(root, catalog)
                with self.assertRaises(ValueError):
                    safe_path("../escape.gb")
        finally:
            server.shutdown()
            server.server_close()
            thread.join()


def run_suites(options):
    prepare(options.offline, options.source, options.site_url)
    filters = {"all": "validation_", "blargg": "validation_blargg_regressions",
               "samesuite": "validation_samesuite_regressions", "mooneye": "validation_mooneye_regressions"}
    runs = [(filters[options.suite], None)] if options.suite != "audio" else [
        (filters["blargg"], "sound/"), (filters["samesuite"], None)]
    for name, case_filter in runs:
        env = os.environ.copy()
        if case_filter: env["SHINING_TEST_ROM_FILTER"] = case_filter
        command = ["cargo", "test", "-p", "app", name, "--locked"]
        if options.offline: command.append("--offline")
        test_args = options.test_args
        if test_args[:1] == ["--"]: test_args = test_args[1:]
        subprocess.run(command + ["--", "--include-ignored", "--nocapture"] + test_args,
                       cwd=REPO, env=env, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("prepare", "run"):
        command = commands.add_parser(name)
        command.add_argument("--offline", action="store_true", help="verify/use cached fixtures without network")
        command.add_argument("--source", choices=["site", "upstream"], default="site")
        command.add_argument("--site-url", help="developer override for checking a local or staged website")
        if name == "run":
            command.add_argument("suite", nargs="?", choices=["all", "audio", "blargg", "samesuite", "mooneye"], default="all")
            command.add_argument("test_args", nargs=argparse.REMAINDER)
    command = commands.add_parser("export")
    command.add_argument("--offline", action="store_true")
    command.add_argument("--output", type=Path, default=UPLOAD)
    command.add_argument("--archive", type=Path, default=REPO / "target/rom-catalog-upload.zip")
    commands.add_parser("check-site", help="check the export and import it through localhost HTTP")
    options = parser.parse_args()
    if options.command == "prepare": prepare(options.offline, options.source, options.site_url)
    elif options.command == "run": run_suites(options)
    elif options.command == "export":
        export(options.output, options.offline)
        options.archive.parent.mkdir(parents=True, exist_ok=True)
        archive = shutil.make_archive(str(options.archive.with_suffix("")), "zip", options.output)
        print("Upload archive: " + archive)
    else: unittest.main(argv=[sys.argv[0]])


if __name__ == "__main__":
    try: main()
    except subprocess.CalledProcessError as error: raise SystemExit(error.returncode)
    except (ValueError, OSError) as error: raise SystemExit(str(error))
