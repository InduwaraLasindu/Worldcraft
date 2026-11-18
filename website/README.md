# Worldcraft website

Public project website for the native Rust application. Its source lives in this
repository under `website/`; GitHub Pages serves the packaged static files at
https://induwaralasindu.github.io/Worldcraft/.

## Publication

The **Worldcraft website** workflow packages and verifies changes on standard
GitHub-hosted Ubuntu runners. Pushes to `main` publish only after those checks
pass. Pull requests run the same checks without publishing. The workflow can also
be run manually.

The repository's **Settings → Pages → Build and deployment → Source** must be
**GitHub Actions**. Initial enablement requires a repository administrator; the
workflow's short-lived token intentionally has no administration permission.

There is no application build, package installation or browser installation on
the owner's computer. No paid services or external runtime APIs are used.
The Rust application and its distribution workflows remain independent.

## Files and checks

- `index.html`, `styles.css`, `app.js`: responsive page, accessible gallery
  tabs and mobile navigation.
- `tools/package-site.mjs`: copies the static source and the three existing
  application captures into the runner's `_site/`.
- `tools/check-site.mjs`: verifies deployable files, anchors and project paths.
- `tools/site-browser-check.mjs`: runs hosted Chromium checks for gallery
  clicks and keyboard navigation, mobile menu and Escape, images, download links,
  no-JavaScript fallback, custom 404 recovery, horizontal overflow at
  320–1440 px, and automated WCAG A/AA rules with axe.
- The workflow saves desktop/mobile screenshots and the accessibility report in
  its `website-browser-review` artifact. These checks are not a full manual
  accessibility audit.

## Content and images

Features and limits are grounded in the repository README, GENERATION.md,
ROADMAP.md and docs/ALPHA2_AUDIT.md. Screenshots are actual application output
from docs/media, visibly labelled as hosted software-rendered captures. The hero
and desktop gallery crop those captures for presentation; they are not generated
product images or gaming-GPU benchmarks.

Download links target the verified published Windows release
`v0.2.0-alpha.2-build.2`, including its installer, portable ZIP and checksums.
Update the copy, version, file sizes and URLs together when a new release is
published. This alpha is explicitly labelled, including its unsigned installer,
finite AI region and unfinished features.

No analytics, cookies, sign-in, remote fonts or client-side API calls are required.
The editable Canva design generation attempt was unavailable because the brand's
administrator disabled that feature; the implemented page uses original code and
existing repository captures.
