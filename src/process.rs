//! ## Example
//!
//! ```no_run
//! use pwntools::process::Process;
//!
//! let mut conn = Process::new(&"./some_binary")?;
//! conn.send(&b"x".repeat(32))?;
//! conn.send(&0x1337beef_u64.to_le_bytes())?;
//! conn.interactive()?;
//! # Ok::<_, std::io::Error>(())
//! ```

use std::ffi::OsStr;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread::sleep;

use crate::util::{Payload, P64};

pub struct Process {
    child: Child,
    stdin: ChildStdin,
    // stdout: ChildStdout,
    reader: BufReader<ChildStdout>,
}

// TODO: しまさんお願いします！
// TODO: PayloadとP64に対してこれをimplする
// @ﾅｶｼｭﾝ: ところで返すのがVecならToVec/to_vecの方が自然かも
// @Hakuyume: 確かに過ぎますね！
// Rust1.53からclippyがto_hoge()系の関数に&selfではなくselfを要求するようになってません？
pub trait ToVec {
    fn to_vec(&self) -> Vec<u8>;
}

impl ToVec for P64 {
    fn to_vec(&self) -> Vec<u8> {
        // self: &P64
        // self.0: u64
        // struct S(T0, T1, T2);
        // .0: T0
        // .1: T1
        // struct S {
        //    _0: T0,
        //    _1: T1,
        //    _2: T2,
        // };
        // ちなみにtuple構造体のメモリ配置は仕様にないので
        // S(T, T, T) が [T; 3] と同じかは運ゲー (コンパイラの気持ち次第)
        self.0.to_le_bytes().to_vec()
    }
}

impl ToVec for Payload {
    fn to_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl ToVec for Vec<u8> {
    fn to_vec(&self) -> Vec<u8> {
        self.clone()
    }
}

impl<const N: usize> ToVec for [u8; N] {
    fn to_vec(&self) -> Vec<u8> {
        self[..].to_vec()

        // [..]は全要素からなるsliceを返す
        // [..5]ってやれば先頭5要素
        // [5..]ってやれば6個目以降
        // あとは [T; N]: AsRef<[T]> なので
        // self.as_ref() でsliceに変換するという手もある
        // ただas_refの変換先が一意でないケースが結構多くでコンパイラに型注釈を要求されたりしてメンドウなのでメンドウ
        // https://hashrust.com/blog/arrays-vectors-and-slices-in-rust/

        // AsRef
        // [Char, Char, Char] -> &[Char]
        //                    -> &str
        //                    -> &Path
        // AsRef::<[u8]>::as_ref(self) こうやるとコンパイラも許してくれる (でもself[..]の方が楽…と僕は思います (個人の感想です))

        // AsRef::as_ref<'a>(&'a self) -> &'a U

        // いろんなAsRefを実装している例はStringとか
        // https://doc.rust-lang.org/std/string/struct.String.html#impl-AsRef%3C%5Bu8%5D%3E
        // ここを見るとわかるとおり String はstrにも[u8]にもOsStrにもPathにもas_refできる
        // このケースだと不便に思えるが
        // std::fs::File::openとかはAsRef<Path>なものを何でも引数にとれるようになっているので
        // std::fs::File::open("path") も std::fs::File::open(format!("{}/{}", "aaa", "bbb")) も書ける (前者はstrで後者はStringを渡しているが大丈夫)

        // self.iter().cloned().collect()はあまり効率が良くない
        // [T; N] の要素はメモリ上に連続していることが保証されているので (そしてVecも要素が連続していることを要求している)
        // Vecの領域をheapに確保 -> memcpyで一括コピー
        // で済む
        // iterしてからcollectするんだと
        // 最初の要素をコピー、次の要素をコピー、みたいなのが走る (ぶっちゃけコンパイラの最適化でなんとかなる説はある)
        // iterが有効なケースとしてはコピー元がHashMapとかLinked listみたいに要素がメモリ上に連続していないケース

        // 似たケースとしては Vec::extendというのがあって、これは任意のiteratorを引数にとれる (つまり元の要素がメモリ上に連続していることを仮定しない)
        // 元の要素がメモリ上で連続しているなら Vec::extend_from_sliceとかを使うとmemcpy一発になって良い
    }
}

// @大天才しまさん: あとは頼みます
// https://blog.rust-lang.org/2021/02/26/const-generics-mvp-beta.html

impl Process {
    pub fn new<S>(program: S) -> io::Result<Self>
    where
        S: AsRef<OsStr>,
    {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);
        Ok(Self {
            child,
            stdin,
            reader,
        })
    }

    // data: Dを要求しているが、Dのto_vecを呼ぶだけならば&Dで十分
    // たとえば呼び出し側がVecを持っていたときにDを要求されると呼び出し側で一旦cloneをしてからこの関数に渡す必要があり無駄
    // &Dなら参照を渡すだけで済む
    // ちなみにもうちょっと凝ったことをするならimpl std::io::Write for Processを実装して
    // 呼び出し側は write!(&mut process, ほげ) みたいにすると更にメモリ効率が良い
    // 今の実装だとどこかでVec用のメモリを確保する必要がある (たとえばP64をwriteするときには一旦元の値の8bytesとは別に8bytes確保する必要がある)
    // write!(...) を使うと逐次書き出しができるのでちょっと良い (新規のバイト列のためにヒープを持つ必要がなくなる)
    //
    // あとwriteln!(...) でsendlineもできるよ
    //
    // write!(&mut std::io::Write, format string, args, args, args)
    // こうじゃなくて write!(&mut std::io::Write, "{}", format!(format, args, args, args))
    ///
    // write!(&mut std::io::Write, arg)
    // write!(&mut std::io::Write, arg)
    // write!(&mut std::io::Write, arg)

    // write! macroはここにある通りwrite_fmtを呼んでいるだけ
    // https://doc.rust-lang.org/std/macro.write.html
    // std::io::Write::write_fmtはここにあるとおり
    // write_strを実装したAdaptorを定義してstd::fmt::writeに渡している
    // https://doc.rust-lang.org/src/std/io/mod.rs.html#1563-1595
    // std::fmt::writeはArgumentsの要素をループして、順番にwrite_strしている
    // https://doc.rust-lang.org/src/core/fmt/mod.rs.html#1077
    // たとえば write!(&mut f, "{} {}", "hello", 24); とかしたときは
    // "hello" を書く
    // " " を書く
    // "24" を書く
    // みたいになるので巨大bufferを確保し直してwriteするよりもメモリ効率は良い

    // 一方でwrite_strが直接ファイルシステムのwriteを呼ぶんだとsystem callモリモリという問題はあって
    // Rust的には「勝手にbufferingとかしないので必要ならユーザーで入れてね」という方針
    // std::io::BufWriterというやつがbufferingをしてくれたりする
    pub fn send<D: ToVec>(&mut self, data: &D) -> io::Result<()> {
        self.stdin.write_all(&data.to_vec())?;
        self.stdin.flush()
    }

    pub fn sendline<D: ToVec>(&mut self, data: &D) -> io::Result<()> {
        self.send(data)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()
    }

    // https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line
    // こういうのあるけど黙っとこ
    // ちなみにBufReadはtraitでBufReaderはBufReadの実装例 (の一つ) ですね
    // BufReadを実装したオレオレ型を定義することはできて、
    // たとえばこんなのとか https://github.com/Hakuyume/lazy-seek
    pub fn recvline(&mut self) -> io::Result<Vec<u8>> {
        // Result<T, E> が基本形
        // でもだいたいどのライブラリも Result<T> = Result<T, 自分のエラー> してくれている
        // たとえば std::io::Result<T> = Result<T, std:io::Error>
        // どっちを使うかは趣味
        // もう絶対std::io::Errorしか使わない! とかなら強気に use std::io::Result; しちゃうという手もある (そうするとResult<T>だけです済むようになる)
        // using namespace std; するかどうかと似ているかも (僕はしない派です)

        // 僕（なかむら）は面倒なので use anyhow::Result で全部まとめちゃう人です anyhow::Result = Result<T, impl 'static + std::error::Error + Send + Sync> <- https://cha-shu00.hatenablog.com/entry/2020/12/08/060000 がわかりやすかった
        // Rustのエラー処理は基本的にある型の値を返すので
        // たとえば型ErrorAを返す関数と型ErrorBを返す関数を呼ぼうと思うと全体としては ErrorAとErrorBの直和になる
        // ナイーブな方法としては
        // enum Error {
        //    A(ErrorA),
        //    B(ErrorB),
        // }
        // みたいな直和型を定義して impl From<ErrorA> for Error と impl From<ErrorB> for Errorを実装することになる
        // でもこれはメンドイ
        // これを楽にする方法がいくつかあってthiserrorとanyhowというcrateでそれぞれ提供されている
        // thiserrorがやってくることはimpl From ~ の部分を勝手にやってくれるだけ
        // なのでErrorの定義とかは自分でしないといけない
        // anyhowはtrait objectという機能を利用して、およそどんなError型からも変換できるというのを実現している
        // なのでとりあえず戻り値を anyhow::Result<T> ってやっておけば幸せになれる
        // 一方でanyhowが戻り値の場合、どういうエラーが含まれているかはわからないので
        // 真面目なライブラリを作るときはthiserrorとかで「ErrorAとErrorBが返ってくるんだな」というのを明示した方が親切という説はある
        // let mut buf = vec![];
        // self.reader.read_line(&mut buf)?;
        // Ok(buf)

        self.recvuntil(b"\n")
    }

    // pub fn recvuntil(&mut self, pattern: &[u8]) -> Result<Vec<u8>, &str> {
    // ここの&strは"dame" のことなのでlifetimeとしては &'static strになって欲しさがある (string literalはstaticな領域に置かれるので (.textだったか.dataだったか))
    // ただlifetimeを省略して書くと
    // pub fn recvuntil<'a, 'b>(&'a mut self, pattern: &'b [u8]) -> Result<Vec<u8>, &'a str> {
    // みたいに解釈される (はず)
    // staticよりもaの方が短いのでstaticなdataをaとして返すのはOK
    // この場合はエラーを保持しておいてProcessが消えたあとで使おうとするとコンパイラが怒ると思う
    // ﾅｶｼｭﾝオススメのanyhowにanyhow::bail!ってのがあるので、それに乗り換えてもいい
    pub fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>> {
        let mut result = vec![];

        let mut buf = [0; 1];
        while self.reader.read_exact(&mut buf).is_ok() {
            result.extend_from_slice(&buf);
            // https://doc.rust-lang.org/std/primitive.slice.html#method.ends_with
            // sliceにends_withってある…
            if result.ends_with(pattern) {
                return Ok(result);
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "dame"))
    }

    pub fn interactive(self) -> io::Result<()> {
        println!("interactive.");
        let mut stdin = self.stdin;

        std::thread::spawn(move || std::io::copy(&mut std::io::stdin(), &mut stdin).unwrap());
        let mut stdout = self.reader;

        std::thread::spawn(move || std::io::copy(&mut stdout, &mut std::io::stdout()).unwrap());

        dbg!(self.child.wait_with_output()?.status);
        Ok(())
    }
}
