# Have I Been Pwned Downloader

Inspired by https://github.com/HaveIBeenPwned/PwnedPasswordsDownloader

---

haveibeenpwned-downloader is a cli tool to download all Pwned Passwords hash ranges and save them offline 
so they can be used without a dependency on the k-anonymity API.

# Installation 

With cargo:
```shell
cargo install haveibeenpwned-downloader
```

Or just grab a release from [release page](https://github.com/alisa101rs/haveibeenpwned-downloader/releases).

# Usage 

```shell
Usage: haveibeenpwned-downloader [OPTIONS]

Options:
  -o, --output <OUTPUT>  Output of the program, can be stdout, or file [default: -]
  -f, --format <FORMAT>  Output format, can be text or binary. Only affects `file` output [default: text] [possible values: text, binary]
  -s, --sorted           Whether output should be sorted
  -h, --help             Print help
  -V, --version          Print version
```

## Download all SHA1 hashes to a single txt file called pwnedpasswords.txt
```shell
haveibeenpwned-downloader -o pwnedpasswords.txt
```

## Download all SHA1 hashes to a single binary file called pwnedpasswords.bin
```shell
haveibeenpwned-downloader -f binary -o pwnedpasswords.txt 
```

## Download all SHA1 hashes and output them to stdout in sorted order
```shell
haveibeenpwned-downloader -s -o -
```

# Binary Format

Binary output format writes file that is just a sequence of items, where each item is:
```text 
0                        20                28
|------------------------|-----------------| 
|   password sha1 hash   |    prevalence   |
|------------------------|-----------------| 
```
