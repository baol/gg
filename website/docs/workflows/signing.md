---
title: Sign commits and tags
description: Sign with your SSH or GPG key and check the signatures on others' commits.
---

# Sign commits and tags

Git Good signs and verifies with Git's own settings, so the key you use on the command line works here: an SSH key (`gpg.format ssh`) or a GPG key (the default, as in Pro Git's *Signing Your Work*).

## Set up your key

Set `user.signingkey` once, for example:

```sh
git config --global user.signingkey ~/.ssh/id_ed25519.pub
git config --global gpg.format ssh
```

or, for GPG, `git config --global user.signingkey <key id>`. **Preferences › Commit** shows the format, key and allowed-signers file Git Good read, and **Edit in repository config…** opens the repository's config in your editor.

## Sign

- **Commits:** the **Sign** checkbox under the commit message starts from `commit.gpgsign`. If signing fails (no key, a locked agent), nothing is committed; Git Good never falls back to an unsigned commit.
- **Tags:** **Sign tag** in [New tag](tags.md).

## Check a signature

Select a commit and open **Signature details…** from its details. The banner says what Git reported:

| Banner | Meaning |
|---|---|
| *Good signature* | Valid, by a key you trust (in allowed signers, for SSH) |
| *Unknown key* | Valid, but the key is not one you trust yet |
| *Bad signature* | The content does not match the signature |
| *Unsigned* | No signature at all |

The rows below name the signer, key, trust and format. The check is read-only.

## From a script

```sh
git good --cli commit --sign -m "Pin the toolchain"
git good --cli signature verify v0.2.0
```
