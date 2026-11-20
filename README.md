# 100マス計算

たし算・ひき算・かけ算を練習する、WebとWindows向けの100マス計算アプリです。
縦と横に0〜9がランダムに並びます。ひき算は「左の数 − 上の数」で、負の答えも入力できます。

- 最初の答えを入力すると計測が始まります。
- Enter・Tabで次のマスへ、矢印キーで上下左右に移動できます。
- 途中でも採点でき、正解・不正解・未入力の件数が表示されます。
- 「新しい問題」を押すか計算の種類を変えると、新しい問題に切り替わります。

問題生成と採点は共通のRustライブラリにあります。Web版はWebAssemblyとブラウザ標準のHTML・CSS・JavaScript、Windows版はWin32 APIで画面を作っています。

## Cloudflare Pages

PagesのGit連携でこのリポジトリを選択し、以下の値を設定します。

| 設定 | 値 |
| --- | --- |
| フレームワーク プリセット | なし |
| ビルド コマンド | `bash scripts/build-web.sh` |
| ビルド出力ディレクトリ | `dist` |

ローカルでビルドした `dist/` を、Pagesのダイレクトアップロードで公開することもできます。
静的なファイル配信だけで動作します。

## Web版のローカル実行

```sh
bash scripts/build-web.sh
python3 -m http.server 8000 --directory dist
```

ブラウザで [http://localhost:8000](http://localhost:8000) を開きます。

## Windows版のビルド

Windowsに [Rust](https://rustup.rs/) と、Visual Studio Build Toolsの「C++ によるデスクトップ開発」ワークロードを入れてください。

```powershell
cargo build --locked --release --bin hundred-math
.\target\release\hundred-math.exe
```

GitHub Actionsのビルドが成功すると、実行結果からWindows用の `100math-windows-x64` と、Cloudflare Pagesにアップロードできる `100math-web` をダウンロードできます。

## テスト

```sh
cargo test --locked
```
