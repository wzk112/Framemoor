# Third-party notices

Framemoor's GPL-3.0-only license applies to original project code. Third-party dependencies keep their respective licenses. Exact versions are recorded in `Cargo.lock`, `desktop/src-tauri/Cargo.lock`, and `desktop/package-lock.json`; those lockfiles and the resource preparation script are part of the corresponding source.

## ExifTool

[ExifTool](https://exiftool.org/) is Copyright 2003–2026 Phil Harvey. Its README licenses it under the same terms as Perl itself (the Perl Artistic License or GPL). The preparation script pins version 13.59, source revision `2200871d9cef988051d2a99d67df3bda6cbb30a8`, from [the upstream repository](https://github.com/exiftool/exiftool/tree/2200871d9cef988051d2a99d67df3bda6cbb30a8).

The script copies the upstream `LICENSE`, `README`, and `lib` with the executable script into the application resource folder. These files must be retained when packaging. Windows builds additionally require the complete official Windows distribution, including its supporting files and notices; do not redistribute only `exiftool.exe`.

## UI and runtime dependencies

The app uses React (MIT), Tauri (MIT or Apache-2.0), and Lucide (ISC, with Feather portions under MIT). Other Rust and JavaScript dependencies are identified by the lockfiles and their upstream package metadata. Retain applicable copyright and license notices when distributing their code or a compiled application. This repository currently publishes source; it does not yet provide public application binaries.

### Lucide

ISC License

Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.

Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

## 中文说明

本项目许可证适用于原创代码，依赖保留各自许可证。ExifTool 的来源、固定版本和许可证随资源脚本保留；Windows 打包需要完整官方发行包及其声明。将来分发应用二进制时，还需随包保留所有适用的第三方版权与许可声明，并提供 GPL 要求的对应源码。本仓库当前发布源码，未提供公开应用安装包。
