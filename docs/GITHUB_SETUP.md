# Publishing kannada-vedic-ime on GitHub (beginner steps)

You need: a free GitHub account, the unzipped project folder, and (optional but easiest)
**GitHub Desktop** from https://desktop.github.com.

## A. Prepare the folder (once)

1. Use the project folder. (`Cargo.lock`, the list of exact dependency versions, is not in the
   repository yet. See "Save Cargo.lock" at the end: you add it once.)
2. Do **not** upload the `target` folder. The included `.gitignore` already skips it.
3. `LICENSE-MIT` already carries your name.

## B. Create the repository

1. Sign in at github.com. Click **+** (top right) → **New repository**.
2. Name: `kannada-vedic-ime`. Description: *Phonetic Kannada typing with Vedic accents (Windows tray app, web typer, Arkavathi font).*
3. Choose **Public**. Leave "Add a README / .gitignore / licence" all **off** (the project already has them).
4. Click **Create repository**.

## C. Upload the project with GitHub Desktop

1. Open GitHub Desktop and sign in.
2. **File → Add local repository…** → choose the project folder → it says "this directory does not appear to be a Git repository" → click **create a repository**.
3. Name `kannada-vedic-ime`, leave other settings, click **Create repository**.
4. In the left list check that `target` is **not** listed and `Cargo.lock` **is**.
5. Bottom left: summary `First release 0.3.0` → **Commit to main**.
6. Click **Publish repository** (top), untick "Keep this code private", click **Publish**.

## D. Turn on the web typer (GitHub Pages)

1. On the repository page: **Settings → Pages**.
2. Source: **Deploy from a branch**. Branch: `main`, folder: `/docs`. **Save**.
3. After about a minute the page is at `https://YOUR-NAME.github.io/kannada-vedic-ime/`.
4. Put that link in the repository's **About** box (gear icon, top right of the repo page) and add topics: `kannada`, `ime`, `vedic`, `rust`.

## E. Check the automatic tests

Open the **Actions** tab. The workflow "CI" runs on every upload and tests on Windows and Linux.
A green tick means it passed. If it is red, click it and send me the error text.

## F. Make the first downloadable release

1. Repository page → **Releases → Draft a new release**.
2. **Choose a tag** → type `v0.3.0` → **Create new tag**.
3. Title `0.3.0`; paste that version's section from `CHANGELOG.md`; click **Publish release**.
4. The **Release** workflow (Actions tab) builds `kannada_ime.exe` and attaches
   `kannada-vedic-ime-v0.3.0-windows.zip` to the release after about five minutes.
   That zip has the program, the Arkavathi font, a one-page quick start and the licences.
5. Share the link of the release page. Users unzip, install the font, double-click the exe.

## Later releases

Change the version in `Cargo.toml`, add a section to `CHANGELOG.md`, commit and push in GitHub Desktop,
then repeat step F with the next tag (`v0.6.0`, …).

### Releasing 0.5.0 (what to do with the new files)

1. Copy the new project files over your repository folder (replace when asked). Do not delete `.git`.
   Delete the two old files `fonts/Kaveri.ttf` and `docs/Kaveri.pdf` (they are replaced by `Arkavathi.ttf` and `Arkavathi.pdf`).
2. GitHub Desktop, **Changes** tab: check the list, summary `Release 0.5.0`, **Commit to main**, **Push origin**.
3. Actions tab: wait for the green tick on "CI" (it now has a Linux job too). If it is red, send me the error text.
4. Pages updates by itself after the push (about a minute).
5. Releases, **Draft a new release**, tag `v0.5.0`, title `0.5.0`, paste the 0.5.0 section of `CHANGELOG.md`,
   **Publish release**. The Windows zip and the Linux `.deb` appear on the release page after about five minutes.
6. Search Console: open the Pages address, press **Verify**, then **Request indexing**.

### Save Cargo.lock (once, after the first green CI run)

1. Actions tab → click the latest green "CI" run → scroll to **Artifacts** → download `Cargo.lock`.
2. Unzip it and put `Cargo.lock` in the repository folder (next to `Cargo.toml`).
3. GitHub Desktop → commit `Add Cargo.lock` → Push. From now on builds use exactly those versions.

## Good to know

* Windows SmartScreen warns about unsigned programs. The quick start explains "More info → Run anyway".
  A code-signing certificate removes the warning but costs money.
* The workflows were written without being able to run them. If one fails on the first try, the log
  shows exactly which line; send it to me.
