# Hosting the ROM catalog

The origin is **https://shininggrimace.com**. The app and developer scripts
read it from `test-roms/site-config.json`; the fixed path is
`/home-of-8-bit/rom-catalog/`.

## Files ready to upload

The complete upload tree is:

```
target/rom-catalog-upload/home-of-8-bit/rom-catalog/
├── index.html
├── directory.json
├── README.txt
└── v1/validation-v7.0-1/
    ├── in-app.json
    ├── headless.json
    ├── licenses/
    │   ├── Collection-MIT.txt
    │   ├── Mooneye-MIT.txt
    │   └── SameSuite-X11.txt
    └── roms/
        ├── mooneye-test-suite/...
        └── same-suite/apu/...
```

`target/rom-catalog-upload.zip` contains that directory's **contents**, ready to
extract at the web path. Upload the contents into your site's
`home-of-8-bit/rom-catalog/` directory, preserving the `v1` hierarchy. The tree
contains **181 ROMs**, supporting **273 headless hardware cases**.
The in-app catalog is a subset of **34 hardware entries using 26 ROMs**. Files
shared across hardware targets are stored once. Blargg ROMs and screenshots
are deliberately absent from both published catalogs and the upload archive.

The human directory is `index.html`; it credits the original suite authors and
compiled collection, links their upstream sources, and links all three complete
license files included in the ZIP. The machine-readable directory is
`directory.json`. A checked-in copy of the latter is
`test-roms/site-directory.json`, with catalog copies in `site-in-app.json` and
`site-headless.json` for review, integrity pins and transport tests. Do not
upload `test-roms/catalog.json`: that developer manifest also lists Blargg.

## Static server configuration

No server-side code, API key, authentication or directory-listing feature is
required. Serve these files over HTTPS:

| Extension | Content-Type |
| --- | --- |
| `.json` | `application/json` |
| `.gb` | `application/octet-stream` |
| `.txt` | `text/plain; charset=utf-8` |
| `.html` | `text/html; charset=utf-8` |

Avoid transforming JSON or text responses: the clients verify exact file
contents. Standard HTTP compression is fine because clients hash the decoded
body. Native desktop/Android clients do not require CORS headers.

Allow revalidation of `directory.json` and `index.html`. Versioned `v1/...`
files may be cached for a long time because they are immutable. Upload those
files first, then the directory/index last. Keep older versions and their
directory records when adding releases so older app versions still work.

## Directory and version scheme

`directory.json` has `schema_version: 1` and a `releases` array. Each release
identifies its two catalogs and license files by relative path and SHA-256.
Catalog entries describe ROM paths, hardware requirements and SHA-256 hashes.
Headless entries also carry non-default reporting protocols and time budgets. ROM URLs are constructed as:

```
<base_url>v1/<release>/roms/<case.path>
```

Both clients select the release named in `site-config.json`; they never
silently upgrade tests to an unreviewed `latest` version. The app verifies the
published release against the checked-in directory and its exact in-app
selection. Both the app and preparation script verify catalogs, ROMs and notices
using SHA-256. Changing a selection requires a new release ID and reviewed
baselines, not replacing files beneath an existing version.

## Rebuild the upload package

With the existing developer cache:

```sh
python3 scripts/test-roms.py export --offline
```

From a fresh checkout, omit `--offline`. The exporter bootstraps from the pinned
upstream compiled collection, verifies it, and publishes only permitted files.
It refuses to overwrite an unrelated directory or change existing versioned
files; give changed content a new release ID. Custom output locations:

```sh
python3 scripts/test-roms.py export --offline \
  --output /tmp/rom-catalog --archive /tmp/rom-catalog.zip
```

Original notices live in `test-roms/licenses/`. ROM preparation inputs are in
the ignored `.test-roms/validation/` developer cache, which includes Blargg and
**must not be uploaded wholesale**. The generated package is the upload source.

## Verify before and after upload

```sh
python3 scripts/test-roms.py check-site
```

This checks the generated inventory, all file hashes, Blargg exclusion, and a
fresh headless import through a temporary local HTTP server. It also verifies
that corrupt downloads are rejected. It needs the prepared developer cache and
permission to bind a localhost port.

Once files are hosted, check the remote preparation path:

```sh
python3 scripts/test-roms.py prepare --source site \
  --site-url https://YOUR-DOMAIN/home-of-8-bit/rom-catalog/
python3 scripts/test-roms.py run --offline
```

Until the real domain is configured, bootstrap tests from upstream instead:

```sh
python3 scripts/test-roms.py run --source upstream
```

The default online test command uses your website for Mooneye/SameSuite and
the pinned upstream archive for Blargg. `--offline` only verifies and uses
cached fixtures, without requiring the archive or contacting either host.

In the app, enable **Game Boy validation ROMs** in Settings → ROM Providers.
It requests the directory/catalog when syncing and downloads a ROM and its
license on first use. Subsequent loads can use the verified cache offline.
The cache retains each hardware target and the fresh-start diagnostic behavior.
