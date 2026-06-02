use super::language::CodeLanguage;

pub(super) fn is_keyword(language: CodeLanguage, token: &str) -> bool {
    word_list_contains(keyword_words(language), token)
}

pub(super) fn is_type_keyword(language: CodeLanguage, token: &str) -> bool {
    word_list_contains(type_words(language), token)
}

pub(super) fn is_builtin(language: CodeLanguage, token: &str) -> bool {
    word_list_contains(builtin_words(language), token)
}

fn word_list_contains(list: &str, token: &str) -> bool {
    list.split_ascii_whitespace().any(|word| word == token)
}

fn keyword_words(language: CodeLanguage) -> &'static str {
    match language {
        CodeLanguage::Rust => "async await const crate else enum false fn for if impl in let match mod move mut pub return self struct trait true type use where while",
        CodeLanguage::TypeScript | CodeLanguage::JavaScript => "async await break case catch class const else export extends false for from function if import interface let new null return this throw true try type undefined var",
        CodeLanguage::Python => "and as assert async await class def elif else except False finally for from if import in is lambda None not or pass raise return True try while with yield",
        CodeLanguage::Go => "break case chan const defer else fallthrough for func go if import interface map nil package range return select struct switch type var",
        CodeLanguage::Java | CodeLanguage::Kotlin | CodeLanguage::Swift | CodeLanguage::CSharp => "abstract async await class const else enum extends false final for fun func if import interface let new null object override private protected public return static struct super switch this throw true try val var void when while",
        CodeLanguage::C | CodeLanguage::Cpp => "auto break case class const else enum extern false for if inline namespace new nullptr private protected public return sizeof static struct switch template this true typedef typename using virtual void while",
        CodeLanguage::Ruby => "and begin break case class def do else elsif end ensure false for if in module nil not or rescue return self super true unless while yield",
        CodeLanguage::Php => "abstract array as break case catch class const else extends false final foreach function if implements interface namespace new null private protected public return static switch throw trait true try use",
        CodeLanguage::Shell => "case do done elif else esac export fi for function if in local readonly return set then until while",
        CodeLanguage::Sql => "ALTER AND AS BY CREATE DELETE DROP FROM GROUP HAVING INSERT INTO JOIN LEFT LIMIT NOT NULL ON OR ORDER RIGHT SELECT SET TABLE UPDATE VALUES WHERE",
        CodeLanguage::Lua => "and break do else elseif end false for function if in local nil not or repeat return then true until while",
        CodeLanguage::Dart | CodeLanguage::Scala => "abstract async await case catch class def else enum extends false final for if import match new null object override return sealed super switch this throw trait true try val var while with yield",
        CodeLanguage::Json | CodeLanguage::Toml | CodeLanguage::Yaml | CodeLanguage::Html | CodeLanguage::Css => "true false null auto block flex grid inherit initial unset",
    }
}

fn type_words(language: CodeLanguage) -> &'static str {
    match language {
        CodeLanguage::Rust => "bool char f32 f64 i32 i64 isize str String u32 u64 usize Option Result Vec",
        CodeLanguage::TypeScript | CodeLanguage::JavaScript => "Array Boolean Date Map Number Object Promise Record Set String boolean number string unknown void",
        CodeLanguage::Python => "bool bytes dict float int list set str tuple",
        CodeLanguage::Go => "bool byte error float32 float64 int int32 int64 rune string uint uint32 uint64",
        CodeLanguage::C | CodeLanguage::Cpp | CodeLanguage::CSharp | CodeLanguage::Java => "bool boolean byte char double float int long short string String uint",
        _ => "",
    }
}

fn builtin_words(language: CodeLanguage) -> &'static str {
    match language {
        CodeLanguage::Rust => "println format vec Some None Ok Err unwrap expect",
        CodeLanguage::TypeScript | CodeLanguage::JavaScript => {
            "console log JSON parse stringify fetch setTimeout map filter reduce then catch"
        }
        CodeLanguage::Python => "print len range open enumerate zip map filter list dict set",
        CodeLanguage::Go => "fmt Println Printf Errorf make new panic recover",
        CodeLanguage::Shell => "echo cd pwd grep sed awk cat find xargs printf",
        _ => "",
    }
}
