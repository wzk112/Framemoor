# Repository instructions

- All changes go through a topic branch and pull request, including docs and CI. Never push commits directly to `main`, force-push it, or weaken/bypass its protections.
- Run relevant checks before requesting a merge. Do not merge with failing required CI. Use squash merge after resolving review conversations.
- Keep README.md and README.zh-CN.md consistent. UI copy is concise and functional in Chinese and English.
- Do not commit personal media, real task records, absolute user paths, secrets, local configuration, generated bundles, or `work/` contents. Tests use synthetic fixtures.
- Sources are read-only. Do not overwrite existing destinations, weaken path/volume identity checks, or claim unverified copies are verified.
- Do not add persistent scan/metadata/checksum caching without an explicit product decision.
- Copy, resume, timestamp, path, and verification changes require relevant regression coverage. Use scratch fixtures; do not run writes against a user's media or card.
- Distinguish unit/browser/native/hardware verification in reports. Windows GUI and additional camera formats need real validation before claiming support.
- Original project code is GPL-3.0-only. Preserve dependency notices; do not change licensing without explicit authorization.
