# 100マス計算

100マス計算で12人のCPUと対戦できる、WebとWindows向けの計算ゲームです。
たし算・ひき算・かけ算を選べます。ひき算は「左の数 − 上の数」で、負の答えも入力できます。

## 対戦

1. 相手と計算の種類を選び、「対戦開始」を押します。
2. 黄色のマスの答えを入力し、Enterまたは「攻撃」で送ります。正解すると次のマスに進み、連続正解で攻撃が強くなります。間違えたら連鎖が途切れ、同じマスにもう一度答えられます。
3. 相手もカウントダウンが0になるたびに攻撃します。先に相手の体力を0にすると勝ちです。

対戦相手は、ミジンコ、ありんこ、ネズミ、うさぎ、赤ちゃん、女の子、灘高生、東大生、ヒュパティア、ピタゴラス、フェルマー、ラマヌジャンです。この順に相手の体力や攻撃の速さが上がります。

## 数学者コイン図鑑

対戦に勝つたびに10ポイントが加算され、200枚そろうまで古代から現代へ順番に1枚ずつ獲得できます。
「コイン図鑑」では時代順に数学者の名前・生没年・業績の一言解説を閲覧でき、獲得済みの顔入りコインが表示されます。コインの肖像は画像生成によるイラストです。

Web版は同じサイトを開いたブラウザに、Windows版は現在のWindowsユーザーに進み具合を自動保存します。

## れんしゅう

100マスを好きな順に解き、途中でも採点できます。Enter・Tabで次のマスへ、矢印キーで上下左右に移動できます。最初の答えを入力すると計測が始まります。

問題生成・採点・対戦ルールは共通のRustライブラリにあります。Web版はWebAssemblyとブラウザ標準のHTML・CSS・JavaScript、Windows版はWin32 APIで画面を作っています。生成した対戦相手のイラストは、WebではWebP、Windowsでは実行ファイルに組み込んだBGRA画像を使用しています。

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
