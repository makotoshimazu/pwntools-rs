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
use std::io::{self, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use crate::util::{Payload, P64};

pub struct Process {
    child: Child,
    stdin: ChildStdin,
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

impl<const N: usize> ToVec for &[u8; N] {
    fn to_vec(&self) -> Vec<u8> {
        // self.iter().cloned().collect()

        self[..].to_vec()

        // これはあまり効率が良くない
        // [T; N] の要素はメモリ上に連続していることが保証されているので (そしてVecも要素が連続していることを要求している)
        // Vecの領域をheapに確保 -> memcpyで一括コピー
        // で済む
        // iterしてからcollectするんだと
        // 最初の要素をコピー、次の要素をコピー、みたいなのが走る (ぶっちゃけコンパイラの最適化でなんとかなる説はある)
        // iterが有効なケースとしてはコピー元がHashMapとかLinked listみたいに要素がメモリ上に連続していないケース
    }
}

// @しまさん: あとは頼みます
// https://blog.rust-lang.org/2021/02/26/const-generics-mvp-beta.html

impl Process {
    pub fn new<S>(program: S) -> io::Result<Self>
    where
        S: AsRef<OsStr>,
    {
        let mut child = Command::new(program).stdin(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take().unwrap();
        Ok(Self { child, stdin })
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
        self.stdin.write_all(&data.to_vec())
    }

    pub fn sendline<D: ToVec>(&mut self, data: &D) -> io::Result<()> {
        self.send(data)?;
        self.stdin.write_all(b"\n")
    }

    pub fn interactive(mut self) -> io::Result<()> {
        let mut stdin = self.stdin;
        std::thread::spawn(move || std::io::copy(&mut std::io::stdin(), &mut stdin).unwrap());
        self.child.wait()?;
        Ok(())
    }
}
