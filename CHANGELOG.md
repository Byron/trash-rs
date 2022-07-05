# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 2.1.5 (2022-07-05)

### Bug Fixes

 - <csr-id-67244ba2e4c71135b0ab36331dc465615e23211a/> Make chrono a default-enabled optional feature.
   This allows to turn chrono support off without actually affecting the
   ability to trash and restore items.
   `chrono` still has issues to dubious local-time support which relies
   on a c-library function that can cause undefined behaviour as it
   accesses an environment variable in a non-threadsafe fashion.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 40 days passed between releases.
 - 1 commit where understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#39](https://github.com/Byron/trash-rs/issues/39)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#39](https://github.com/Byron/trash-rs/issues/39)**
    - Make chrono a default-enabled optional feature. ([`67244ba`](https://github.com/Byron/trash-rs/commit/67244ba2e4c71135b0ab36331dc465615e23211a))
 * **Uncategorized**
    - improve CI stage names; fix feature configuration on windows ([`5591fda`](https://github.com/Byron/trash-rs/commit/5591fdab131de1f6fa5a04bef44d7b394d3f7f72))
    - silence clippy ([`d13be48`](https://github.com/Byron/trash-rs/commit/d13be48c59a1a0df3e37aa676cda06cc1f48ece9))
    - add rust-cache for faster builds ([`676a43f`](https://github.com/Byron/trash-rs/commit/676a43f7ec7c116a7b40dcf4236bf2156a88fd04))
</details>

## 2.1.4 (2022-05-25)

### Fixes

- upgrade the `windows` crate to v0.37 to resolve [a build issue](https://github.com/Byron/trash-rs/issues/39) and lay the foundation
  for more regular updates of the windows support.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 8 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#39](https://github.com/Byron/trash-rs/issues/39), [#51](https://github.com/Byron/trash-rs/issues/51)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#39](https://github.com/Byron/trash-rs/issues/39)**
    - prepare changelog ([`7816e07`](https://github.com/Byron/trash-rs/commit/7816e07bab38a79aa6f5d705a4fb40f330ac155b))
 * **[#51](https://github.com/Byron/trash-rs/issues/51)**
    - Upgrade windows crate ([`d18f9d4`](https://github.com/Byron/trash-rs/commit/d18f9d435d2f76fb982f4bfcc98d5ccfe57c092c))
 * **Uncategorized**
    - Release trash v2.1.4 ([`17d162f`](https://github.com/Byron/trash-rs/commit/17d162fcf7a53d3d82961a448d4b70b4eb596825))
</details>

## 2.1.3 (2022-05-17)

### Fixes

- include `windows` crate only on windows for reduced CI build times from ~9s to ~4s.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 3 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#5050505050](https://github.com/Byron/trash-rs/issues/5050505050)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#5050505050](https://github.com/Byron/trash-rs/issues/5050505050)**
    - update changelog ([`8e64f34`](https://github.com/Byron/trash-rs/commit/8e64f34bd6f1b823353fae61d60f765615be0024))
 * **Uncategorized**
    - Release trash v2.1.3 ([`f98bc45`](https://github.com/Byron/trash-rs/commit/f98bc45199cbb24525d2b41c748b9547f3c3ac44))
    - Merge pull request #50 from rgwood/windows-dep ([`883c5a4`](https://github.com/Byron/trash-rs/commit/883c5a48c8ad07bef4f7e1822a31761211cf304d))
    - Add names to CI steps ([`ef7003a`](https://github.com/Byron/trash-rs/commit/ef7003a4f83910f318b05a3f51960a33fd444915))
    - Only use `windows` crate on Windows ([`e088525`](https://github.com/Byron/trash-rs/commit/e088525047a14a531d414fe9cd098e08fe2ff79f))
</details>

## 2.1.2 (2022-05-13)

### Bug Fixes

 - <csr-id-367cf5f2616f1f49b115189b3bede3bb99f8324d/> avoid inconsistency when using relative paths in trashed file info.
   We use absolute paths now without trying to generate a relative path
   based on some top directory as the latter seems to be causing
   inconsistencies on some linux distros, as the restore path ends
   up being incorrect.
   
   Rather go with the absolute truth and don't fiddle with path
   transformations at all to make it work everywhere.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 days passed between releases.
 - 1 commit where understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#39](https://github.com/Byron/trash-rs/issues/39)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#39](https://github.com/Byron/trash-rs/issues/39)**
    - avoid inconsistency when using relative paths in trashed file info. ([`367cf5f`](https://github.com/Byron/trash-rs/commit/367cf5f2616f1f49b115189b3bede3bb99f8324d))
 * **Uncategorized**
    - Release trash v2.1.2 ([`e0746f0`](https://github.com/Byron/trash-rs/commit/e0746f0df91623231d13531ec33632f03f0588ac))
</details>

## 2.1.1 (2022-05-10)

### Bug Fixes

 - <csr-id-dcda6df8cefa06bf08e7eca7db2c34b050c2d913/> Properly reconstruct paths when restoring files on freedesktop if those were relative.
   
   Previously it would be unable to reconstruct original paths if the trash
   directory was on a mount point due to a 'split brain' of sorts.
   
   When trashing files it would create original path information based
   on them being relative to a mount point, but when restoring them
   it would reconstruct them to be relative to the trash top level
   directory.
   
   Now the reconstruction happens against to mount point itself which makes
   restoration match.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 2 calendar days.
 - 3 days passed between releases.
 - 1 commit where understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#47](https://github.com/Byron/trash-rs/issues/47)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#47](https://github.com/Byron/trash-rs/issues/47)**
    - Properly reconstruct paths when restoring files on freedesktop if those were relative ([`dcda6df`](https://github.com/Byron/trash-rs/commit/dcda6df8cefa06bf08e7eca7db2c34b050c2d913))
    - Somewhat hard-code special case for fedora ([`90f0f9b`](https://github.com/Byron/trash-rs/commit/90f0f9b035678efe51a20d4a47fd09158b8ef455))
    - proper cleanup after potential assertion failure ([`1f3a600`](https://github.com/Byron/trash-rs/commit/1f3a6005eabd4629fe0743030a612a29fcb7d80c))
    - remove unused trait ([`ac913d8`](https://github.com/Byron/trash-rs/commit/ac913d83ed9344d8ed8e18957b2e99136e0b29c1))
 * **Uncategorized**
    - Release trash v2.1.1 ([`50ab31a`](https://github.com/Byron/trash-rs/commit/50ab31afa9f641a16a1ab50bf1ea8f8bacb0330f))
    - update changelog ([`98d32c8`](https://github.com/Byron/trash-rs/commit/98d32c88e85b2b40ea17d372c427ef168ad80b30))
    - more robust removal of test files in failure case on os specific tests ([`3f6502d`](https://github.com/Byron/trash-rs/commit/3f6502db02e09e36c2fbce2fea054a9a2b9229de))
</details>

## 2.1.0 (2022-05-06)

### Fixes

- Leading directories are now created on linux to avoid errors when trashing nested directories.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 103 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#45](https://github.com/Byron/trash-rs/issues/45), [#47](https://github.com/Byron/trash-rs/issues/47)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#45](https://github.com/Byron/trash-rs/issues/45)**
    - reproduce issue with lack of leading directories and fix it ([`d5b6faa`](https://github.com/Byron/trash-rs/commit/d5b6faa81d59ccd6185261399bc7449432b9deb6))
 * **[#47](https://github.com/Byron/trash-rs/issues/47)**
    - Try to reproduce ([`8eba501`](https://github.com/Byron/trash-rs/commit/8eba50155e006cf923d8bb77fea88cde6395512e))
 * **Uncategorized**
    - Release trash v2.1.0 ([`b3a4547`](https://github.com/Byron/trash-rs/commit/b3a45471ce5fcd489a096145e06ac663ed854747))
    - prepare upcoming release ([`e3bbb6b`](https://github.com/Byron/trash-rs/commit/e3bbb6be1072675c331176e8d0585cc67910d17b))
    - Merge branch 'refactor-tests' ([`0e90cac`](https://github.com/Byron/trash-rs/commit/0e90cace515344c68eead8e59180487561849289))
    - Assure tests don't race ([`d9778ba`](https://github.com/Byron/trash-rs/commit/d9778ba1912c5764cbfaa9c46b2bba5c3d1899eb))
    - thanks clippy ([`220a216`](https://github.com/Byron/trash-rs/commit/220a2164e86bf7f0e1e636d24595b6ce4182de14))
    - Move all intergration tests into corresponding location ([`e5dc62e`](https://github.com/Byron/trash-rs/commit/e5dc62ee2b363a11e57e4aad2c1d128d2f8961e2))
</details>

## 2.0.4 (2022-01-23)

We detected the possibility of UB in the Linux and FreeBSD versions of `get_mount_points()` and reduced the likelihood
of it happening in a multi-threaded environment by synchronizing access. You can read more about the state of
a more permanent fix [in the tracking issue](https://github.com/Byron/trash-rs/issues/42).

All previous 2.0.* releases which contained this function were yanked from crates-io.

### Fixes

* Make internal `get_mount_points()` thread-safe to reduce chance of UB greatly. 
  This may reduce performance of crates that are using trash from multiple threads somewhat, as a part of the operation
  is now synchronized.
* Fix build on FreeBSD, handle UB similarly to the above.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 30 calendar days.
 - 30 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release trash v2.0.4 ([`c7edcb1`](https://github.com/Byron/trash-rs/commit/c7edcb175dd125bda5b15e726fc7b36eae3c89a4))
    - Prepare changelog for next release ([`b65f574`](https://github.com/Byron/trash-rs/commit/b65f574d5aeb8ea3a918e8288c8d13dd082b8f0a))
    - Add Mutex to linux version of get_mount_points(); document UB chance in lib.rs ([`c5c9c5e`](https://github.com/Byron/trash-rs/commit/c5c9c5e40d345736df7d078bf8e6991acc701e83))
    - Use Mutex to prevent concurrent access to getmntinfo ([`5c8e0ce`](https://github.com/Byron/trash-rs/commit/5c8e0ce1c700c68fc63c612cc0ea5b3191f6b0d1))
    - Merge pull request #43 from wezm/num-threads-freebsd ([`8f10c85`](https://github.com/Byron/trash-rs/commit/8f10c852bd9ec2e69353a0dd5397fab1c4ba089f))
    - Fix build on FreeBSD after refactor ([`f3d31e5`](https://github.com/Byron/trash-rs/commit/f3d31e54dd93c22605e8178958a1caa503be19f4))
    - Use `num_threads()` to avoid UB in FreeBSD version of get_mount_points() ([`3c153ae`](https://github.com/Byron/trash-rs/commit/3c153ae2f1ed92d8a240a742e90fcb0e483284b8))
    - refactor ([`92ab7b9`](https://github.com/Byron/trash-rs/commit/92ab7b91adcde3305cc3e319fb0b59feff8f81cc))
    - Add BSD compatible implementation of get_mount_points ([`82d2132`](https://github.com/Byron/trash-rs/commit/82d2132f8e1323272f5d8e1f54112589f75c3202))
    - Run `cargo-diet` for a more minimal crates package ([`561f21d`](https://github.com/Byron/trash-rs/commit/561f21d9de2a56cb0f0c87002d2ead3dc8ca6ab2))
</details>

## 2.0.3 (2021-12-23)

### Bug Fixes

 - <csr-id-cb5b6176aa296853f7a6e3cfa177e1235acaa903/> let dependency specification in Cargo.toml match cfg directives in code
   This fixes [issue 40](https://github.com/Byron/trash-rs/issues/40).

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 125 calendar days.
 - 125 days passed between releases.
 - 1 commit where understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#37](https://github.com/Byron/trash-rs/issues/37), [#40](https://github.com/Byron/trash-rs/issues/40)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#37](https://github.com/Byron/trash-rs/issues/37)**
    - fix some clippy warnings ([`3c566ef`](https://github.com/Byron/trash-rs/commit/3c566ef417350b75e02ea80be51165815014ec74))
 * **[#40](https://github.com/Byron/trash-rs/issues/40)**
    - let dependency specification in Cargo.toml match cfg directives in code ([`cb5b617`](https://github.com/Byron/trash-rs/commit/cb5b6176aa296853f7a6e3cfa177e1235acaa903))
 * **Uncategorized**
    - Release trash v2.0.3 ([`6864e34`](https://github.com/Byron/trash-rs/commit/6864e340890f247f675982744396bae8ea856565))
    - Disable lint for platforms where it matters ([`b4add86`](https://github.com/Byron/trash-rs/commit/b4add8643cc0659b4318f3113a197794cb0032b0))
    - update changelog with `cargo changelog` ([`932cea4`](https://github.com/Byron/trash-rs/commit/932cea48c6ceba2adf0b824c3236b330e232de12))
    - Add Rust CI status badge ([`b94fce2`](https://github.com/Byron/trash-rs/commit/b94fce2bf74dd5c1ee66735eca32d6ace5db83ea))
</details>

## v2.0.2 (2021-08-18)

### Changed

- Fix failing to delete files on some freedesktop (eg Linux) systems when the home was not mounted at the root.
- The `list` function now returns an empty list if there is no trash directory (it used to return an error).
- Fix for test failing on Linux environments that don't have a desktop environment (more specifically don't have a tool like `gio`)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 104 calendar days.
 - 108 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#34](https://github.com/Byron/trash-rs/issues/34), [#35](https://github.com/Byron/trash-rs/issues/35), [#36](https://github.com/Byron/trash-rs/issues/36)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#34](https://github.com/Byron/trash-rs/issues/34)**
    - Fix for failing to delete files on Freedesktop systems (eg Linux) ([`bd8679c`](https://github.com/Byron/trash-rs/commit/bd8679c39b163e87c33bec7a669cebdc9ff37358))
 * **[#35](https://github.com/Byron/trash-rs/issues/35)**
    - Fix for test failing on some Linux environments ([`9da7b59`](https://github.com/Byron/trash-rs/commit/9da7b590a23940693ad2809ca28c7ec904a574a6))
 * **[#36](https://github.com/Byron/trash-rs/issues/36)**
    - Avoid error from the list function ([`cb59c7e`](https://github.com/Byron/trash-rs/commit/cb59c7e09f6409881c24131bf25cb89930203655))
 * **Uncategorized**
    - Update version ([`600b59c`](https://github.com/Byron/trash-rs/commit/600b59c3422d5f6f51aca27b867a64650f06c865))
    - Update windows-rs ([`2b64f38`](https://github.com/Byron/trash-rs/commit/2b64f3832781b2715688c236194392ec31b2c5d3))
    - Some minor improvements ([`0e281bc`](https://github.com/Byron/trash-rs/commit/0e281bcbfe0bb50d8b68782cdd1da7d7e74355f7))
    - Merge pull request #29 from ArturKovacs/update-win-rs ([`2a1eaf8`](https://github.com/Byron/trash-rs/commit/2a1eaf8630b2c49b06e28d323d85e95dd0dd514a))
    - Revert the build script ([`1b4a501`](https://github.com/Byron/trash-rs/commit/1b4a501685fa02e80579fa825156ec1077a39519))
    - Ran cargo fmt ([`42884ae`](https://github.com/Byron/trash-rs/commit/42884aec20b1ad1b59213b465b34b600a8bf4cff))
    - Update windows-rs and fix for cross compilation ([`681d7b4`](https://github.com/Byron/trash-rs/commit/681d7b49140c0fd1db33628ee66bf432a5818eac))
</details>

## v2.0.1 (2021-05-02)

### Changed

- Fix not being able to trash any item on some systems.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 11 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update version number ([`6f11f8d`](https://github.com/Byron/trash-rs/commit/6f11f8dd58190afd00b15211584c15919477ad07))
    - Merge pull request #26 from ArturKovacs/fix-25 ([`13a36ce`](https://github.com/Byron/trash-rs/commit/13a36cec736c8127676f90f45f0c3941590aca1d))
    - Update changelog ([`812b574`](https://github.com/Byron/trash-rs/commit/812b574f08c73b3b26cd3c1b4b761e209f9544df))
    - Fix for error when trashing an item ([`a876d0f`](https://github.com/Byron/trash-rs/commit/a876d0f92e48cae89ac4815187b0bdff7634148d))
</details>

## v2.0.0 (2021-04-20)

### Changed

- The "Linux" implementation was replaced by a custom Freedesktop implementation.

### Added

- `list`, `purge_all`, and `restore_all` to Windows and Freedesktop

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 32 commits contributed to the release over the course of 4 calendar days.
 - 86 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #11 from ArturKovacs/v2-dev ([`3dcac24`](https://github.com/Byron/trash-rs/commit/3dcac248a029a7f78d41fcdf1645f7ad6dc5bc4d))
    - Merge branch 'v2-dev' of https://github.com/ArturKovacs/trash-rs into v2-dev ([`e9047a3`](https://github.com/Byron/trash-rs/commit/e9047a364ba531c720d356867649d13dcac1f918))
    - Update the version number ([`28307a6`](https://github.com/Byron/trash-rs/commit/28307a662c99b150268d7b20d946ee9bd51baa75))
    - Add test for NsFileManager delete method ([`8aea6ef`](https://github.com/Byron/trash-rs/commit/8aea6ef92cde4daa6336424c91192d80ca62bde6))
    - Run cargo fmt ([`3ce2160`](https://github.com/Byron/trash-rs/commit/3ce2160a87de0b88901048063cc5d7b5aa8455f2))
    - Fix clippy error ([`2158550`](https://github.com/Byron/trash-rs/commit/21585507786f8ab0426afbe2dab7dd738b6c8c84))
    - More tweaks ([`ee2527a`](https://github.com/Byron/trash-rs/commit/ee2527a78134e408f73347b4f6bfaf43d2f9fb29))
    - Minor tweaks ([`1c43fe7`](https://github.com/Byron/trash-rs/commit/1c43fe7e7de12e1ecd551633b62c6105cfa4019d))
    - Update readme, add changelog ([`4c1ece3`](https://github.com/Byron/trash-rs/commit/4c1ece3db523de11546f487efc8ae39b01b35b5c))
    - Add more logging to the freedesktop implementation ([`a94b4ce`](https://github.com/Byron/trash-rs/commit/a94b4ce160a4926d3cf777517ee6768a364b8310))
    - Remove the Filesystem error kind ([`1138d8c`](https://github.com/Byron/trash-rs/commit/1138d8ccbc6e5bfd84daca86aacd9902326ecd3a))
    - Don't run the CI for the nightly Rust ([`afa33ba`](https://github.com/Byron/trash-rs/commit/afa33badbab2473f649881d78c9acc49de376697))
    - Fix clippy error ([`a182fbc`](https://github.com/Byron/trash-rs/commit/a182fbc7cd685151ff109c6a76623e27c2f666af))
    - Fix freedesktop errors ([`afd17c3`](https://github.com/Byron/trash-rs/commit/afd17c3efd939283d20d2e130cfeb4b609adad42))
    - Update the list example ([`a18d055`](https://github.com/Byron/trash-rs/commit/a18d055e684589eb2f9176ae6536ec433b023dc1))
    - Fix warnings on macOS ([`f12cea9`](https://github.com/Byron/trash-rs/commit/f12cea96221d52c3809408009894fa28ac3b8a0c))
    - Tweaked tests and documentation ([`330b1ec`](https://github.com/Byron/trash-rs/commit/330b1ec4f99376666ed48fb125e95e1928b1be0d))
    - Documentation improvements ([`18337bf`](https://github.com/Byron/trash-rs/commit/18337bf73c3e89a90e793ba9b6e9741d558a019a))
    - Rename `extra` to `os_limited` and other tweaks ([`29b6b11`](https://github.com/Byron/trash-rs/commit/29b6b113ffa4af0eeab6b147f5e19cee605e3274))
    - Update the macOS backend ([`eff82e4`](https://github.com/Byron/trash-rs/commit/eff82e4e11195eb6871a08b5f607e9a0ab921a4c))
    - Update the macos backend ([`e739014`](https://github.com/Byron/trash-rs/commit/e739014e08c4b883ff9350cbc1beef0e2da10797))
    - Removed the silly PlatformApi error ([`61fa667`](https://github.com/Byron/trash-rs/commit/61fa667246cc83960d68f6ccfe2f29080ddb4186))
    - Implement restore_all for windows ([`baa5171`](https://github.com/Byron/trash-rs/commit/baa5171c83a9f007e55dc2c2f412b7aad08815cc))
    - Implement purge_all on Windows ([`9fc224d`](https://github.com/Byron/trash-rs/commit/9fc224db47b3bd4c34d6a34c3251dc495a076fe9))
    - Remove the WinNull workaround ([`d8ab41f`](https://github.com/Byron/trash-rs/commit/d8ab41f97a25fdf81a20f0641c8a472e948a7f35))
    - Ran cargo fmt ([`cf13e78`](https://github.com/Byron/trash-rs/commit/cf13e78e303da0a99fb01c801a030a9f1ff9d8af))
    - Implement the `list` function for windows ([`6e77795`](https://github.com/Byron/trash-rs/commit/6e777954438acf6db2f0089d6a34fa0b77a60ab1))
    - Implemented the delete function using `windows-rs` ([`218d0d0`](https://github.com/Byron/trash-rs/commit/218d0d00492833fdb301aeaaf1164b837a2a3af4))
    - Fix example ([`69dbe38`](https://github.com/Byron/trash-rs/commit/69dbe386af48a1fb3d7a85fbb09235f263a9a5a2))
    - Don't track the lockfile ([`942108d`](https://github.com/Byron/trash-rs/commit/942108d378c92edf387788d572f91808683b0019))
    - Merge branch 'master' into v2-dev ([`c2d7a35`](https://github.com/Byron/trash-rs/commit/c2d7a35b7584f17e724853e6e3fdff9efeff5835))
    - Minor adjustments ([`314e808`](https://github.com/Byron/trash-rs/commit/314e80823fccbb2b78558e9bea2f086e77fba26a))
</details>

## v1.3.0 (2021-01-24)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 154 calendar days.
 - 165 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fix for clippy error ([`a728dce`](https://github.com/Byron/trash-rs/commit/a728dce614add4f3aa10c2b2721a4eb2a9e57cca))
    - Increment version and update fmt ([`6d2270a`](https://github.com/Byron/trash-rs/commit/6d2270a0cbcebd8ebcc67a0278f81271e355bc63))
    - Ran fmt and fix for warning ([`ff7cf3b`](https://github.com/Byron/trash-rs/commit/ff7cf3b09916c04ff861047db2b5005621d0597a))
    - Fix for path canonicalization ([`5dfe5dc`](https://github.com/Byron/trash-rs/commit/5dfe5dc0beaa29a537808d017e5852ad976644e4))
    - Merge pull request #23 from cbr9/optimize--get-desktop-environment ([`c887b6b`](https://github.com/Byron/trash-rs/commit/c887b6bdbe707320aada2478e5033f101e86aba6))
    - optimized get_desktop_environment() ([`a0a7fbb`](https://github.com/Byron/trash-rs/commit/a0a7fbbcd3e0e60b4b59066b65f3f4443ab57dbf))
    - Update readme ([`30427f0`](https://github.com/Byron/trash-rs/commit/30427f04121bfbd8526d06deaf1d04cc7db145b0))
    - Oops that Path wasn't completely unused after all ([`ba850ee`](https://github.com/Byron/trash-rs/commit/ba850eee27299e2aca0b1fc634f566f91a40e43b))
    - Fixed compile warning and ran rustfmt ([`c556d28`](https://github.com/Byron/trash-rs/commit/c556d284887ae72bf95b6fafba357a59f982204d))
    - Removed Cargo.lock from the gitignore. ([`cdde3a7`](https://github.com/Byron/trash-rs/commit/cdde3a7a34671b9ba26231361319f19459b75567))
    - Implement `delete` and `delete_all` for macOS ([`cb564ef`](https://github.com/Byron/trash-rs/commit/cb564ef6efcd770cf96c527624da38b14db4b6ff))
    - Updated readme ([`7a298be`](https://github.com/Byron/trash-rs/commit/7a298be45e22943206617eff9fbc2eca1234223c))
    - Implement `delete` and `delete_all` for windows. ([`d9a25c8`](https://github.com/Byron/trash-rs/commit/d9a25c8f6addf87eb177184f24a444835fad0b4a))
    - Add `delete` functions for Linux ([`fedeb83`](https://github.com/Byron/trash-rs/commit/fedeb8350625f252510ae5a2c5bb26fb74876b49))
    - Update to the readme, incorporating some suggestions by Caleb Bassi ([`9bddccc`](https://github.com/Byron/trash-rs/commit/9bddccc2e8e368f7278135e60d41da601fa20aa4))
    - Merge pull request #18 from cjbassi/rename-files ([`c49b496`](https://github.com/Byron/trash-rs/commit/c49b4961b1e83548777ea0a24cd99c7e6c6660fe))
    - rename readme and license files ([`5a9a5a6`](https://github.com/Byron/trash-rs/commit/5a9a5a66b53803b037636febc9265b66bcfc7334))
    - Adds a deprecated attribute to the `is_implemented` function. ([`386db96`](https://github.com/Byron/trash-rs/commit/386db96e8eebed0b60d79ac055e8e312f01a605c))
</details>

## v1.1.0 (2020-08-12)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 87 days passed between releases.
 - 0 commits where understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#17](https://github.com/Byron/trash-rs/issues/17)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#17](https://github.com/Byron/trash-rs/issues/17)**
    - Implement std::error::Error for trash::Error ([`8765acf`](https://github.com/Byron/trash-rs/commit/8765acf6ef7a93db322baabb40df1edfc405b437))
 * **Uncategorized**
    - Increment minor version number ([`281bb93`](https://github.com/Byron/trash-rs/commit/281bb931159f22da85f4f23fcee92cc96e8a28e7))
</details>

## v1.0.1 (2020-05-16)

<csr-id-576fad719cb240203dec030890d54fe416a42edd/>

### Refactor

 - <csr-id-576fad719cb240203dec030890d54fe416a42edd/> port mac implementation to work with v2
   Updates the existing Mac implementation to compile with v2 of the
   library. Does not add any new functionality other defining required
   methods.
   
   Tests fail for methods relating to `list`, `purge_all`, or
   `restore_all`, which are unimplemented.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 59 commits contributed to the release over the course of 198 calendar days.
 - 218 days passed between releases.
 - 1 commit where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update readme and increment the patch field in the version number. ([`217b473`](https://github.com/Byron/trash-rs/commit/217b4739d84827744a0b23a23b344e2118ac6f5b))
    - Merge pull request #15 from myfreeweb/bsd ([`5af79ab`](https://github.com/Byron/trash-rs/commit/5af79aba26d767b1be1c816d18ff3ef7a7b3301d))
    - Build "linux" module on *BSD (any non-macOS unix) ([`9e38ff8`](https://github.com/Byron/trash-rs/commit/9e38ff8ea89a70c9c3c87369413b3f798baa727d))
    - Fix clippy warnings on linux. ([`0731a64`](https://github.com/Byron/trash-rs/commit/0731a6403f0854ee7098d4cd534bb435684ffd50))
    - Merge branch 'master' of https://github.com/ArturKovacs/trash ([`1e43dc1`](https://github.com/Byron/trash-rs/commit/1e43dc1682e95ff497d6ce5d4b3185336227506f))
    - Add .vscode to gitignore ([`f63f7ce`](https://github.com/Byron/trash-rs/commit/f63f7ce308dd3dad6c64e39d210891cc237704a8))
    - Fix for clippy warning. ([`80eba00`](https://github.com/Byron/trash-rs/commit/80eba00727a72e47b016cda028a6ede7d218085f))
    - Ran `cargo fmt`. ([`b753689`](https://github.com/Byron/trash-rs/commit/b7536891e17e796ab17a4070100024f1754e3ba0))
    - Add tests for Windows and MacOS as well as the nightly toolchain. ([`29876c7`](https://github.com/Byron/trash-rs/commit/29876c7f96144c82f8bee565c80d433beaba548d))
    - Default rust workflow (GitHub Actions) ([`cf7d22f`](https://github.com/Byron/trash-rs/commit/cf7d22fd2db44f52bbc48041edee66be391b2911))
    - Merge branch 'master' into v2-dev ([`32de332`](https://github.com/Byron/trash-rs/commit/32de3324618619ffefe0cc7ed3c16bb8df647caf))
    - Merge branch 'master' into v2-dev ([`b3ea819`](https://github.com/Byron/trash-rs/commit/b3ea819c48c619ff0b41df8357da0c4cd1da8d67))
    - Remove Azure test ([`3e7db4f`](https://github.com/Byron/trash-rs/commit/3e7db4f45f81232567b9beb65860dd4a8651a6fa))
    - Add GitHub Actions test ([`0c8b1fa`](https://github.com/Byron/trash-rs/commit/0c8b1fa561a574678d4db38fc3f0a10b470aca38))
    - Fix wording in Readme ([`7025e10`](https://github.com/Byron/trash-rs/commit/7025e102110a00a290619a32822a7bbd823e3925))
    - Update Readme to reflect the state of development. ([`a31a944`](https://github.com/Byron/trash-rs/commit/a31a94487c46e8ac2b886f59b79b5aa0be195fb8))
    - Merge pull request #9 from NilsIrl/patch-1 ([`02bb739`](https://github.com/Byron/trash-rs/commit/02bb73950db23156680a2273498a1e815ab6fa3d))
    - Fix typo ([`6c4d650`](https://github.com/Byron/trash-rs/commit/6c4d650fd8dd2346e20d37a17d6db9e4afd2ce77))
    - Added test cases and extra documentation. The test cases cover empty input for  `purge_all` and `restore_all`. ([`c275449`](https://github.com/Byron/trash-rs/commit/c275449adde03da9d7ba3064f4b72c9e816c2979))
    - Moved Linux and Windows specific features to a mod ([`0e2fc93`](https://github.com/Byron/trash-rs/commit/0e2fc93f8bb5c17a9c5c89849453b04344995cab))
    - Refined windows implementation. Added error kind `RestoreTwins`. ([`d105553`](https://github.com/Byron/trash-rs/commit/d1055538c16e318247b2817cce85ec522b2163a2))
    - Ran `cargo fmt` ([`743b2f3`](https://github.com/Byron/trash-rs/commit/743b2f3f889b0519275b9d52475b0b68c661382d))
    - Merge branch 'v2-mac' into v2-dev ([`62a7218`](https://github.com/Byron/trash-rs/commit/62a7218644511d75e7b4d162c70aac2f1a4625e9))
    - Merge branch 'v2-dev' of https://github.com/ArturKovacs/trash into v2-dev ([`90d3fa6`](https://github.com/Byron/trash-rs/commit/90d3fa62eb7ca525e6fc45c48cdeba95322a7416))
    - Removed the two previously added errors. Replaced `ZeroMountPointsFound` and `CantOpenMountPointsFile` with `panic!` after coming across https://lukaskalbertodt.github.io/2019/11/14/thoughts-on-error-handling-in-rust.html and reading http://joeduffyblog.com/2016/02/07/the-error-model/ ([`fa03282`](https://github.com/Byron/trash-rs/commit/fa0328204662ea871336087738642a6c9dece33e))
    - No need for those parentheses ([`534677e`](https://github.com/Byron/trash-rs/commit/534677ed4c23316f956dc43e84f16ca68f744331))
    - Merge branch 'v2-dev' of https://github.com/ArturKovacs/trash into v2-dev ([`00fc235`](https://github.com/Byron/trash-rs/commit/00fc235235d4f00e34ffb8cd04b975157037ab91))
    - Add missing error kinds. `ZeroMountPointsFound` and `CantOpenMountPointsFile` were added. ([`6c79f7d`](https://github.com/Byron/trash-rs/commit/6c79f7d95604c68363cd710f99150e655152dbc4))
    - Improve collision handling and add collision test. ([`6db249b`](https://github.com/Byron/trash-rs/commit/6db249bef9529eab4f8325dbc36e84a47000525b))
    - Merge pull request #7 from ArturKovacs/v2-linux ([`b957f38`](https://github.com/Byron/trash-rs/commit/b957f3894b6d06b42cc48824a125825b954496c4))
    - Remove debug lines. ([`3ab9217`](https://github.com/Byron/trash-rs/commit/3ab9217280b6656b4a861e61cc590c326c008d67))
    - Fix creating the home trash folder. ([`10364c6`](https://github.com/Byron/trash-rs/commit/10364c64508acd4fb564cc761bc746eb2d9dd4b1))
    - Create home trash if doesn't yet exist. Also added debug print line numbers. ([`e1b2aae`](https://github.com/Byron/trash-rs/commit/e1b2aaeb8fd00d07be250208767ba85217d55010))
    - Attempt to add RUST_BACKTRACE=1 again. ([`7bd6023`](https://github.com/Byron/trash-rs/commit/7bd6023710e60ddbf6f9f15bcf1bd714f6aaedd9))
    - Merge branch 'v2-dev' into v2-linux ([`303a274`](https://github.com/Byron/trash-rs/commit/303a274989d0a6b6faf7eab3f0ae9149e6e86d71))
    - Added RUST_BACKTRACE=1 to test. ([`c1cb106`](https://github.com/Byron/trash-rs/commit/c1cb10611be762cb13fce4fd38c39961ad84317a))
    - Merge branch 'v2-dev' into v2-linux ([`ecf521f`](https://github.com/Byron/trash-rs/commit/ecf521fbccaae27b4eb160598448341d5e8b7700))
    - Add ability to trash items from an external drive. ([`16f0ee1`](https://github.com/Byron/trash-rs/commit/16f0ee19beaf116b9dfd44de06b272f8b62fb3fd))
    - Added a partialy implementaiton of `remove_all`. Can't remove from non-root devices or partitions. ([`1e03167`](https://github.com/Byron/trash-rs/commit/1e031679d6a3c0cd8a780014d3331bd4fd8bcd1d))
    - Steps towards implementing `remove_all`. ([`20ba354`](https://github.com/Byron/trash-rs/commit/20ba354399d2ce96e7ee624bbf2a03407df163db))
    - Fix for `list` failing on Linux. This happened because `list` on Linux didn't handle paralell threads manipulating the trash correctly. ([`d6cb6ba`](https://github.com/Byron/trash-rs/commit/d6cb6bac6758a0020b88ef5632bf9f06f748f7ca))
    - Merge pull request #6 from ayazhafiz/refactor/mac2 ([`adf0ea4`](https://github.com/Byron/trash-rs/commit/adf0ea4fab0ace99b443c5498ba5495c89abcd30))
    - Remove the Cirrus CI config. ([`fd597fc`](https://github.com/Byron/trash-rs/commit/fd597fc852eddb23472276be6c638e6e40281f67))
    - port mac implementation to work with v2 ([`576fad7`](https://github.com/Byron/trash-rs/commit/576fad719cb240203dec030890d54fe416a42edd))
    - Add MacOS and Linux as targets for CI tests. ([`e409a98`](https://github.com/Byron/trash-rs/commit/e409a983b64d36c2b585ecdb5374357a34f5da53))
    - Fix OS setup in Azure's config. ([`08db817`](https://github.com/Byron/trash-rs/commit/08db8172080832727bd5002e394054c34c5147ea))
    - Update Azure's target operating systems. ([`cfb25b6`](https://github.com/Byron/trash-rs/commit/cfb25b6e0d9e8fd00379871760430009c52289cd))
    - Add Azure Pipelines CI. ([`760dfa6`](https://github.com/Byron/trash-rs/commit/760dfa64e53a2e1228230c073bd551acb868b286))
    - Add Cirrus CI test. ([`e5c22c4`](https://github.com/Byron/trash-rs/commit/e5c22c4567e2f289a918affc46fa923a962799cf))
    - Added implementation of purge_all for Linux. Also ran rustfmt and created a rustfmt config. ([`a90f9bf`](https://github.com/Byron/trash-rs/commit/a90f9bfa0d19fd6fad88c91bbd0e6a46c4661a0e))
    - Now using the url crate for parsing the original location on Linux. ([`02ffe0b`](https://github.com/Byron/trash-rs/commit/02ffe0b336136c730213670d4c5b6eb04addaa55))
    - Add `list` implementation for linux. ([`5c73fea`](https://github.com/Byron/trash-rs/commit/5c73fea22341c520103564953535c84b7271fc4e))
    - Add note about coming features in version 2 to the Readme. ([`dddbe25`](https://github.com/Byron/trash-rs/commit/dddbe25171e6f93ffd2b80627d25e8313ff21498))
    - Improve the Error type and add `create_remove_empty_folder` test. ([`a940b66`](https://github.com/Byron/trash-rs/commit/a940b66abea7769aba8e0b1d99995b8174239877))
    - Changed `std::mem::uninit` and `std::mem::zeroed` to `std::mem::MaybeUninit`. Plus ran Rustfmt. ([`632a6fb`](https://github.com/Byron/trash-rs/commit/632a6fb31fc4c4bee751f0537ca317fe9f933f5c))
    - Now `purge_all` doesn't show a dialog on windows. ([`07e3bc2`](https://github.com/Byron/trash-rs/commit/07e3bc25832a49af13ee3c2a1fdc1f425fce8805))
    - Fix `purge_all` and `restore_all` reading invvalid memory and not executing the operation on the requested items. Add test cases for `purge_all` and `restore_all`. Test are now thread safe. ([`22d5181`](https://github.com/Byron/trash-rs/commit/22d51813759c129e87625eef5d068a1481bfbdb8))
    - Implement `purge_all` and `restore_all` for Windows. ([`e06c825`](https://github.com/Byron/trash-rs/commit/e06c825e93ce9774733cfcf539e3c7e928cdb8cc))
    - Run rust fmt. Implement `list` for Windows. ([`3f29636`](https://github.com/Byron/trash-rs/commit/3f29636a978fd5a462db1588040d794d81648be7))
</details>

## v1.0.0 (2019-10-11)

<csr-id-576fad719cb240203dec030890d54fe416a42edd/>

### Refactor

 - <csr-id-576fad719cb240203dec030890d54fe416a42edd/> port mac implementation to work with v2
   Updates the existing Mac implementation to compile with v2 of the
   library. Does not add any new functionality other defining required
   methods.
   
   Tests fail for methods relating to `list`, `purge_all`, or
   `restore_all`, which are unimplemented.

### New Features

 - <csr-id-d68cc2aedee5e8316117bec257975da30cbd7483/> implementation for macOS
   Moves files to trash on macOS by executing an AppleScript command to
   delete all requested paths.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 99 calendar days.
 - 1 commit where understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' where seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Updated version number and readme ([`79ee69e`](https://github.com/Byron/trash-rs/commit/79ee69e3e12a9a66146897ab432f29eaa8ac2d28))
    - Merge pull request #1 from ayazhafiz/feat/mac ([`48a6b11`](https://github.com/Byron/trash-rs/commit/48a6b11cae520ca1b60c42270912402c1d51c018))
    - implementation for macOS ([`d68cc2a`](https://github.com/Byron/trash-rs/commit/d68cc2aedee5e8316117bec257975da30cbd7483))
    - Fix wrong code references in the linux implementation. ([`037fed8`](https://github.com/Byron/trash-rs/commit/037fed8ae6b5ed76cec00037cdc8340d7787d7cb))
    - Add docs badge to readme ([`88261d5`](https://github.com/Byron/trash-rs/commit/88261d5af0b165a06483ea07e5aa378d2223d067))
    - Increment version number ([`f758543`](https://github.com/Byron/trash-rs/commit/f75854358b7c8dea23aec6f40362fab4039d9659))
    - Improve readme. Add remove_all function to mac as unimplemented. ([`2850270`](https://github.com/Byron/trash-rs/commit/2850270004cea47718b18aa3d3b290263ba7b8e3))
    - Merge branch 'master' of https://github.com/ArturKovacs/trash ([`a651d0f`](https://github.com/Byron/trash-rs/commit/a651d0f3b7c261da9bd2fd65f16166b43d63abf3))
    - Add folder remove test ([`b7bb22f`](https://github.com/Byron/trash-rs/commit/b7bb22f7b21027ad73d53eded480921f3346a14c))
    - Updated the Cargo.toml ([`1ec1ef9`](https://github.com/Byron/trash-rs/commit/1ec1ef96b941bbab6672a86b24b0e23afdfe2165))
    - Add license ([`1725e61`](https://github.com/Byron/trash-rs/commit/1725e612e103dda2e574543ea90821934ed46ae6))
    - Add doc comments ([`b16a1d3`](https://github.com/Byron/trash-rs/commit/b16a1d3dff8c3b4d9b24b6dc87c9e3781b667c58))
    - Fix Windows compile error. ([`15e801e`](https://github.com/Byron/trash-rs/commit/15e801e1a242e1b0263fe854e6c9d58a68774dd0))
    - Add the `remove_all` function. ([`f033dc3`](https://github.com/Byron/trash-rs/commit/f033dc308ee061adc579f7426697c5cb3c280956))
    - Minor refactoring. ([`9c7363d`](https://github.com/Byron/trash-rs/commit/9c7363dbeae19edb8079a57fcc93d012c8064ef0))
    - Add Linux support. ([`0429d3b`](https://github.com/Byron/trash-rs/commit/0429d3bf1e7f09e97eae4aefde0cb8c8e283b235))
    - Fixed platform specific compilation ([`84cba5b`](https://github.com/Byron/trash-rs/commit/84cba5b11acf02d5ba99d776529b93bac54f8094))
    - Changed required winapi version. ([`d49bce8`](https://github.com/Byron/trash-rs/commit/d49bce8d346e10c08910520383ed4054a3948535))
    - Initial ([`4c23314`](https://github.com/Byron/trash-rs/commit/4c233148288711419a04fdfa96e36dcb77f0469f))
</details>

