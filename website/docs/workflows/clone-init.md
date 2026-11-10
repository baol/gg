---
title: Clone or create a repository
description: Clone from a URL into your projects folder, or turn a folder into a repository.
---

# Clone or create a repository

Both start from the welcome view or the **Repository** menu.

## Clone

Choose **Repository › Clone…**. If the clipboard holds a repository URL, it is already filled in.

![The Clone repository dialog: Repository URL, Into folder ~/src and Name](../assets/screens/clone.png){ .gg-shot }

- **Into folder** offers your clone folders (Preferences › General) with the preferred one checked, the folder you started Git Good from, and **Choose another folder…**.
- **Name** follows the URL (`acme-api` for `git@github.com:acme/acme-api.git`); the line below shows the full target.
- Progress shows in the footer, and the new repository opens when Git finishes. A folder that exists and is not empty is never touched.

Git Good uses your own Git, so SSH keys and credential helpers work as on the command line. When credentials are missing, Git Good says so instead of waiting for a password prompt.

## Create a repository

Choose **Repository › New repository…**, pick a folder and the first branch name. With **Commit the existing files as "Initial commit"** on, the files already in the folder (minus what `.gitignore` excludes) become the first commit.

![The New repository dialog: Folder, First branch main, and Commit the existing files as "Initial commit"](../assets/screens/new-repository.png){ .gg-shot }

## From a script

```sh
git good --cli clone git@github.com:acme/acme-api.git ~/src/acme-api
git good --cli init --branch main --commit ~/notes
```
