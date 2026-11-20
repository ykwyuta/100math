# 100マス計算

たし算・ひき算・かけ算を練習する、WebとWindows向けの100マス計算アプリです。
縦と横に0〜9がランダムに並びます。ひき算は「左の数 − 上の数」で、負の答えも入力できます。

- 最初の答えを入力すると計測が始まります。
- Enter・Tabで次のマスへ、矢印キーで上下左右に移動できます。
- 途中でも採点でき、正解・不正解・未入力の件数が表示されます。
- 「新しい問題」を押すか計算の種類を変えると、新しい問題に切り替わります。

問題生成と採点は共通のRustライブラリにあります。Web版はWebAssemblyとブラウザ標準のHTML・CSS・JavaScript、Windows版はWin32 APIで画面を作っています。

## Web版のローカル実行

Node.jsとnpmが使える環境で、WebAssemblyをビルドしてWranglerでプレビューします。

```sh
bash scripts/build-web.sh
npx --yes wrangler@4 pages dev
```

ブラウザで [http://localhost:8788](http://localhost:8788) を開きます。

## Cloudflare Pagesへの公開

Wranglerは `wrangler.toml` のプロジェクト名 `100math` と出力先 `dist/` を使用します。
静的ファイルの配信だけで動作します。

Cloudflareにログインし、初回のみPagesプロジェクトを作成します。

```sh
npx --yes wrangler@4 login
npx --yes wrangler@4 pages project create 100math --production-branch master
```

本番に公開するときは、次のコマンドを実行します。

```sh
bash scripts/build-web.sh
npx --yes wrangler@4 pages deploy --branch master
```

すでに同じ名前のPagesプロジェクトがある場合は、プロジェクト作成を省略できます。
別の名前を使う場合は `wrangler.toml` の `name` を変更してください。

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
