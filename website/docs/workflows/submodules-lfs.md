---
title: Submodules and LFS
description: See which submodules differ from what the parent records, and fetch missing LFS objects.
---

# Submodules and LFS

## Submodules

**Repository › Submodules…** lists every submodule with its status and two commits: **RECORDED**, the commit the parent repository stages, and **CHECKED OUT**, the one in the submodule's folder.

- An amber *Different commit* means the submodule is checked out somewhere else than the parent records.
- A grey *Uninitialized* submodule has never been checked out. **Initialize and update…** runs `git submodule update --init` for that path after you confirm; it may fetch from the submodule's URL and never changes the parent's index.
- **Open nested repository** opens the submodule as a repository of its own.

In Review and Local Changes, a submodule change shows the two commit IDs instead of a diff, with the same details under **File actions**.

## Git LFS

Files stored with [Git LFS](https://git-lfs.com) are small pointers in Git and the real content on the LFS server. Git Good reads the pointer and shows the object's ID, size and whether it is on your disk; an image that is available previews like any other image.

**Repository › LFS…** lists the objects your checkout is missing. Check the ones you want and click **Fetch** (++ctrl+enter++): Git Good runs one `git lfs fetch` for them. This needs `git-lfs` installed.

## From a script

```sh
git good --cli submodule status vendor/lib
git good --cli lfs fetch assets/logo.psd
```
