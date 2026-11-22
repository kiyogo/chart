//! Global UI language catalog & runtime bridge (Spec 064).
//!
//! Canonical source language in all source code is English (`en`, index 0).
//! All 197 living world languages are translated cleanly from canonical English via
//! lazy-loaded indexed JSON packs (`/locales/_keys.json` + `/locales/{code}.json`).
//! Right-to-left scripts flip `document.documentElement.dir` to `rtl`, and `translate="no"`
//! locks out browser auto-translate extensions from corrupting native translations.

use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageSpec {
    pub code: &'static str,
    pub native: &'static str,
    pub english: &'static str,
    pub flag: &'static str,
    pub rtl: bool,
}

/// Complete catalog of 197 verified living world languages.
pub const LANGUAGES: &[LanguageSpec] = &[
    LanguageSpec { code: "en", native: "English", english: "English", flag: "gb", rtl: false },
    LanguageSpec { code: "vi", native: "Tiếng Việt", english: "Vietnamese", flag: "vn", rtl: false },
    LanguageSpec { code: "zh-CN", native: "简体中文", english: "Chinese (Simplified)", flag: "cn", rtl: false },
    LanguageSpec { code: "zh-TW", native: "繁體中文", english: "Chinese (Traditional)", flag: "tw", rtl: false },
    LanguageSpec { code: "yue", native: "粵語", english: "Cantonese", flag: "hk", rtl: false },
    LanguageSpec { code: "ja", native: "日本語", english: "Japanese", flag: "jp", rtl: false },
    LanguageSpec { code: "ko", native: "한국어", english: "Korean", flag: "kr", rtl: false },
    LanguageSpec { code: "es", native: "Español", english: "Spanish", flag: "es", rtl: false },
    LanguageSpec { code: "fr", native: "Français", english: "French", flag: "fr", rtl: false },
    LanguageSpec { code: "de", native: "Deutsch", english: "German", flag: "de", rtl: false },
    LanguageSpec { code: "pt-BR", native: "Português (Brasil)", english: "Portuguese (Brazil)", flag: "br", rtl: false },
    LanguageSpec { code: "pt-PT", native: "Português (Portugal)", english: "Portuguese (Portugal)", flag: "pt", rtl: false },
    LanguageSpec { code: "ru", native: "Русский", english: "Russian", flag: "ru", rtl: false },
    LanguageSpec { code: "ar", native: "العربية", english: "Arabic", flag: "sa", rtl: true },
    LanguageSpec { code: "ar-EG", native: "مصري", english: "Egyptian Arabic", flag: "eg", rtl: true },
    LanguageSpec { code: "hi", native: "हिन्दी", english: "Hindi", flag: "in", rtl: false },
    LanguageSpec { code: "bn", native: "বাংলা", english: "Bengali", flag: "bd", rtl: false },
    LanguageSpec { code: "ur", native: "اردو", english: "Urdu", flag: "pk", rtl: true },
    LanguageSpec { code: "id", native: "Bahasa Indonesia", english: "Indonesian", flag: "id", rtl: false },
    LanguageSpec { code: "ms", native: "Bahasa Melayu", english: "Malay", flag: "my", rtl: false },
    LanguageSpec { code: "fil", native: "Filipino", english: "Filipino", flag: "ph", rtl: false },
    LanguageSpec { code: "th", native: "ไทย", english: "Thai", flag: "th", rtl: false },
    LanguageSpec { code: "km", native: "ខ្មែរ", english: "Khmer", flag: "kh", rtl: false },
    LanguageSpec { code: "lo", native: "ລາວ", english: "Lao", flag: "la", rtl: false },
    LanguageSpec { code: "my", native: "မြန်မာ", english: "Burmese", flag: "mm", rtl: false },
    LanguageSpec { code: "tr", native: "Türkçe", english: "Turkish", flag: "tr", rtl: false },
    LanguageSpec { code: "it", native: "Italiano", english: "Italian", flag: "it", rtl: false },
    LanguageSpec { code: "nl", native: "Nederlands", english: "Dutch", flag: "nl", rtl: false },
    LanguageSpec { code: "pl", native: "Polski", english: "Polish", flag: "pl", rtl: false },
    LanguageSpec { code: "uk", native: "Українська", english: "Ukrainian", flag: "ua", rtl: false },
    LanguageSpec { code: "ro", native: "Română", english: "Romanian", flag: "ro", rtl: false },
    LanguageSpec { code: "cs", native: "Čeština", english: "Czech", flag: "cz", rtl: false },
    LanguageSpec { code: "sk", native: "Slovenčina", english: "Slovak", flag: "sk", rtl: false },
    LanguageSpec { code: "hu", native: "Magyar", english: "Hungarian", flag: "hu", rtl: false },
    LanguageSpec { code: "el", native: "Ελληνικά", english: "Greek", flag: "gr", rtl: false },
    LanguageSpec { code: "sv", native: "Svenska", english: "Swedish", flag: "se", rtl: false },
    LanguageSpec { code: "da", native: "Dansk", english: "Danish", flag: "dk", rtl: false },
    LanguageSpec { code: "no", native: "Norsk", english: "Norwegian", flag: "no", rtl: false },
    LanguageSpec { code: "fi", native: "Suomi", english: "Finnish", flag: "fi", rtl: false },
    LanguageSpec { code: "is", native: "Íslenska", english: "Icelandic", flag: "is", rtl: false },
    LanguageSpec { code: "fa", native: "فارسی", english: "Persian", flag: "ir", rtl: true },
    LanguageSpec { code: "he", native: "עברית", english: "Hebrew", flag: "il", rtl: true },
    LanguageSpec { code: "ps", native: "پښتو", english: "Pashto", flag: "af", rtl: true },
    LanguageSpec { code: "ku", native: "Kurdî (Kurmancî)", english: "Kurdish (Kurmanji)", flag: "iq", rtl: false },
    LanguageSpec { code: "ckb", native: "کوردی (سۆرانی)", english: "Kurdish (Sorani)", flag: "iq", rtl: true },
    LanguageSpec { code: "sd", native: "سنڌي", english: "Sindhi", flag: "pk", rtl: true },
    LanguageSpec { code: "bal", native: "بلوچی", english: "Balochi", flag: "pk", rtl: true },
    LanguageSpec { code: "ug", native: "ئۇيغۇرچە", english: "Uyghur", flag: "cn", rtl: true },
    LanguageSpec { code: "yi", native: "ייִדיש", english: "Yiddish", flag: "il", rtl: true },
    LanguageSpec { code: "dv", native: "ދިވެހި", english: "Dhivehi", flag: "mv", rtl: true },
    LanguageSpec { code: "ta", native: "தமிழ்", english: "Tamil", flag: "in", rtl: false },
    LanguageSpec { code: "te", native: "తెలుగు", english: "Telugu", flag: "in", rtl: false },
    LanguageSpec { code: "mr", native: "मराठी", english: "Marathi", flag: "in", rtl: false },
    LanguageSpec { code: "gu", native: "ગુજરાતી", english: "Gujarati", flag: "in", rtl: false },
    LanguageSpec { code: "kn", native: "ಕನ್ನಡ", english: "Kannada", flag: "in", rtl: false },
    LanguageSpec { code: "ml", native: "മലയാളം", english: "Malayalam", flag: "in", rtl: false },
    LanguageSpec { code: "pa", native: "ਪੰਜਾਬੀ", english: "Punjabi", flag: "in", rtl: false },
    LanguageSpec { code: "or", native: "ଓଡ଼ିଆ", english: "Odia", flag: "in", rtl: false },
    LanguageSpec { code: "as", native: "অসমীয়া", english: "Assamese", flag: "in", rtl: false },
    LanguageSpec { code: "ne", native: "नेपाली", english: "Nepali", flag: "np", rtl: false },
    LanguageSpec { code: "new", native: "नेपाल भाषा", english: "Newari", flag: "np", rtl: false },
    LanguageSpec { code: "si", native: "සිංහල", english: "Sinhala", flag: "lk", rtl: false },
    LanguageSpec { code: "sa", native: "संस्कृतम्", english: "Sanskrit", flag: "in", rtl: false },
    LanguageSpec { code: "mai", native: "मैथिली", english: "Maithili", flag: "in", rtl: false },
    LanguageSpec { code: "bho", native: "भोजपुरी", english: "Bhojpuri", flag: "in", rtl: false },
    LanguageSpec { code: "doi", native: "डोगरी", english: "Dogri", flag: "in", rtl: false },
    LanguageSpec { code: "gom", native: "कोंकणी", english: "Konkani", flag: "in", rtl: false },
    LanguageSpec { code: "mni-Mtei", native: "ꯃꯩꯇꯩꯂꯣꯟ", english: "Meiteilon", flag: "in", rtl: false },
    LanguageSpec { code: "lus", native: "Mizo ṭawng", english: "Mizo", flag: "in", rtl: false },
    LanguageSpec { code: "sat", native: "ᱥᱟᱱᱛᱟᱲᱤ", english: "Santali", flag: "in", rtl: false },
    LanguageSpec { code: "kha", native: "Khasi", english: "Khasi", flag: "in", rtl: false },
    LanguageSpec { code: "bo", native: "བོད་ཡིག", english: "Tibetan", flag: "cn", rtl: false },
    LanguageSpec { code: "dz", native: "རྫོང་ཁ", english: "Dzongkha", flag: "bt", rtl: false },
    LanguageSpec { code: "ka", native: "ქართული", english: "Georgian", flag: "ge", rtl: false },
    LanguageSpec { code: "hy", native: "Հայերեն", english: "Armenian", flag: "am", rtl: false },
    LanguageSpec { code: "az", native: "Azərbaycanca", english: "Azerbaijani", flag: "az", rtl: false },
    LanguageSpec { code: "kk", native: "Қазақша", english: "Kazakh", flag: "kz", rtl: false },
    LanguageSpec { code: "uz", native: "Oʻzbekcha", english: "Uzbek", flag: "uz", rtl: false },
    LanguageSpec { code: "ky", native: "Кыргызча", english: "Kyrgyz", flag: "kg", rtl: false },
    LanguageSpec { code: "tk", native: "Türkmençe", english: "Turkmen", flag: "tm", rtl: false },
    LanguageSpec { code: "tg", native: "Тоҷикӣ", english: "Tajik", flag: "tj", rtl: false },
    LanguageSpec { code: "mn", native: "Монгол", english: "Mongolian", flag: "mn", rtl: false },
    LanguageSpec { code: "tt", native: "Татарча", english: "Tatar", flag: "ru", rtl: false },
    LanguageSpec { code: "ba", native: "Башҡортса", english: "Bashkir", flag: "ru", rtl: false },
    LanguageSpec { code: "cv", native: "Чӑвашла", english: "Chuvash", flag: "ru", rtl: false },
    LanguageSpec { code: "ce", native: "Нохчийн", english: "Chechen", flag: "ru", rtl: false },
    LanguageSpec { code: "os", native: "Ирон", english: "Ossetian", flag: "ge", rtl: false },
    LanguageSpec { code: "av", native: "Авар", english: "Avar", flag: "ru", rtl: false },
    LanguageSpec { code: "crh", native: "Qırımtatarca", english: "Crimean Tatar", flag: "ua", rtl: false },
    LanguageSpec { code: "sah", native: "Саха тыла", english: "Yakut", flag: "ru", rtl: false },
    LanguageSpec { code: "bua", native: "Буряад", english: "Buryat", flag: "ru", rtl: false },
    LanguageSpec { code: "kv", native: "Коми", english: "Komi", flag: "ru", rtl: false },
    LanguageSpec { code: "ab", native: "Аԥсуа", english: "Abkhaz", flag: "ge", rtl: false },
    LanguageSpec { code: "bg", native: "Български", english: "Bulgarian", flag: "bg", rtl: false },
    LanguageSpec { code: "sr", native: "Српски", english: "Serbian", flag: "rs", rtl: false },
    LanguageSpec { code: "hr", native: "Hrvatski", english: "Croatian", flag: "hr", rtl: false },
    LanguageSpec { code: "bs", native: "Bosanski", english: "Bosnian", flag: "ba", rtl: false },
    LanguageSpec { code: "sl", native: "Slovenščina", english: "Slovenian", flag: "si", rtl: false },
    LanguageSpec { code: "mk", native: "Македонски", english: "Macedonian", flag: "mk", rtl: false },
    LanguageSpec { code: "sq", native: "Shqip", english: "Albanian", flag: "al", rtl: false },
    LanguageSpec { code: "lt", native: "Lietuvių", english: "Lithuanian", flag: "lt", rtl: false },
    LanguageSpec { code: "lv", native: "Latviešu", english: "Latvian", flag: "lv", rtl: false },
    LanguageSpec { code: "et", native: "Eesti", english: "Estonian", flag: "ee", rtl: false },
    LanguageSpec { code: "be", native: "Беларуская", english: "Belarusian", flag: "by", rtl: false },
    LanguageSpec { code: "ga", native: "Gaeilge", english: "Irish", flag: "ie", rtl: false },
    LanguageSpec { code: "cy", native: "Cymraeg", english: "Welsh", flag: "gb-wls", rtl: false },
    LanguageSpec { code: "gd", native: "Gàidhlig", english: "Scottish Gaelic", flag: "gb-sct", rtl: false },
    LanguageSpec { code: "fo", native: "Føroyskt", english: "Faroese", flag: "fo", rtl: false },
    LanguageSpec { code: "kl", native: "Kalaallisut", english: "Greenlandic", flag: "gl", rtl: false },
    LanguageSpec { code: "eu", native: "Euskara", english: "Basque", flag: "es", rtl: false },
    LanguageSpec { code: "ca", native: "Català", english: "Catalan", flag: "es", rtl: false },
    LanguageSpec { code: "gl", native: "Galego", english: "Galician", flag: "es", rtl: false },
    LanguageSpec { code: "mt", native: "Malti", english: "Maltese", flag: "mt", rtl: false },
    LanguageSpec { code: "lb", native: "Lëtzebuergesch", english: "Luxembourgish", flag: "lu", rtl: false },
    LanguageSpec { code: "fy", native: "Frysk", english: "Frisian", flag: "nl", rtl: false },
    LanguageSpec { code: "co", native: "Corsu", english: "Corsican", flag: "fr", rtl: false },
    LanguageSpec { code: "br", native: "Brezhoneg", english: "Breton", flag: "fr", rtl: false },
    LanguageSpec { code: "oc", native: "Occitan", english: "Occitan", flag: "fr", rtl: false },
    LanguageSpec { code: "scn", native: "Sicilianu", english: "Sicilian", flag: "it", rtl: false },
    LanguageSpec { code: "lmo", native: "Lombard", english: "Lombard", flag: "it", rtl: false },
    LanguageSpec { code: "vec", native: "Vèneto", english: "Venetian", flag: "it", rtl: false },
    LanguageSpec { code: "fur", native: "Furlan", english: "Friulian", flag: "it", rtl: false },
    LanguageSpec { code: "lij", native: "Ligure", english: "Ligurian", flag: "it", rtl: false },
    LanguageSpec { code: "eo", native: "Esperanto", english: "Esperanto", flag: "globe", rtl: false },
    LanguageSpec { code: "la", native: "Latina", english: "Latin", flag: "va", rtl: false },
    LanguageSpec { code: "af", native: "Afrikaans", english: "Afrikaans", flag: "za", rtl: false },
    LanguageSpec { code: "sw", native: "Kiswahili", english: "Swahili", flag: "ke", rtl: false },
    LanguageSpec { code: "am", native: "አማርኛ", english: "Amharic", flag: "et", rtl: false },
    LanguageSpec { code: "ti", native: "ትግርኛ", english: "Tigrinya", flag: "er", rtl: false },
    LanguageSpec { code: "om", native: "Afaan Oromoo", english: "Oromo", flag: "et", rtl: false },
    LanguageSpec { code: "so", native: "Soomaali", english: "Somali", flag: "so", rtl: false },
    LanguageSpec { code: "ha", native: "Hausa", english: "Hausa", flag: "ng", rtl: false },
    LanguageSpec { code: "yo", native: "Yorùbá", english: "Yoruba", flag: "ng", rtl: false },
    LanguageSpec { code: "ig", native: "Igbo", english: "Igbo", flag: "ng", rtl: false },
    LanguageSpec { code: "zu", native: "isiZulu", english: "Zulu", flag: "za", rtl: false },
    LanguageSpec { code: "xh", native: "isiXhosa", english: "Xhosa", flag: "za", rtl: false },
    LanguageSpec { code: "st", native: "Sesotho", english: "Sesotho", flag: "ls", rtl: false },
    LanguageSpec { code: "sn", native: "chiShona", english: "Shona", flag: "zw", rtl: false },
    LanguageSpec { code: "ny", native: "Chichewa", english: "Chichewa", flag: "mw", rtl: false },
    LanguageSpec { code: "rw", native: "Kinyarwanda", english: "Kinyarwanda", flag: "rw", rtl: false },
    LanguageSpec { code: "rn", native: "Ikirundi", english: "Kirundi", flag: "bi", rtl: false },
    LanguageSpec { code: "lg", native: "Luganda", english: "Luganda", flag: "ug", rtl: false },
    LanguageSpec { code: "mg", native: "Malagasy", english: "Malagasy", flag: "mg", rtl: false },
    LanguageSpec { code: "bm", native: "Bamanankan", english: "Bambara", flag: "ml", rtl: false },
    LanguageSpec { code: "ee", native: "Eʋegbe", english: "Ewe", flag: "gh", rtl: false },
    LanguageSpec { code: "ak", native: "Twi", english: "Twi (Akan)", flag: "gh", rtl: false },
    LanguageSpec { code: "ln", native: "Lingála", english: "Lingala", flag: "cd", rtl: false },
    LanguageSpec { code: "nso", native: "Sepedi", english: "Sepedi", flag: "za", rtl: false },
    LanguageSpec { code: "ts", native: "Xitsonga", english: "Tsonga", flag: "za", rtl: false },
    LanguageSpec { code: "tn", native: "Setswana", english: "Tswana", flag: "bw", rtl: false },
    LanguageSpec { code: "ss", native: "SiSwati", english: "Swati", flag: "sz", rtl: false },
    LanguageSpec { code: "ve", native: "Tshivenḓa", english: "Venda", flag: "za", rtl: false },
    LanguageSpec { code: "nr", native: "isiNdebele", english: "Ndebele", flag: "za", rtl: false },
    LanguageSpec { code: "kri", native: "Krio", english: "Krio", flag: "sl", rtl: false },
    LanguageSpec { code: "wo", native: "Wolof", english: "Wolof", flag: "sn", rtl: false },
    LanguageSpec { code: "ff", native: "Fulfulde", english: "Fulani", flag: "sn", rtl: false },
    LanguageSpec { code: "sg", native: "Sängö", english: "Sango", flag: "cf", rtl: false },
    LanguageSpec { code: "kg", native: "Kikongo", english: "Kongo", flag: "cd", rtl: false },
    LanguageSpec { code: "lua", native: "Tshiluba", english: "Tshiluba", flag: "cd", rtl: false },
    LanguageSpec { code: "bem", native: "Ichibemba", english: "Bemba", flag: "zm", rtl: false },
    LanguageSpec { code: "tum", native: "chiTumbuka", english: "Tumbuka", flag: "mw", rtl: false },
    LanguageSpec { code: "ach", native: "Acholi", english: "Acholi", flag: "ug", rtl: false },
    LanguageSpec { code: "luo", native: "Dholuo", english: "Luo", flag: "ke", rtl: false },
    LanguageSpec { code: "ndc-ZW", native: "Ndau", english: "Ndau", flag: "zw", rtl: false },
    LanguageSpec { code: "ber-Latn", native: "Tamaziɣt", english: "Tamazight", flag: "ma", rtl: false },
    LanguageSpec { code: "jv", native: "Basa Jawa", english: "Javanese", flag: "id", rtl: false },
    LanguageSpec { code: "su", native: "Basa Sunda", english: "Sundanese", flag: "id", rtl: false },
    LanguageSpec { code: "ban", native: "Basa Bali", english: "Balinese", flag: "id", rtl: false },
    LanguageSpec { code: "min", native: "Baso Minangkabau", english: "Minangkabau", flag: "id", rtl: false },
    LanguageSpec { code: "ace", native: "Acèh", english: "Acehnese", flag: "id", rtl: false },
    LanguageSpec { code: "mak", native: "Basa Mangkasara", english: "Makassarese", flag: "id", rtl: false },
    LanguageSpec { code: "bbc", native: "Batak Toba", english: "Batak Toba", flag: "id", rtl: false },
    LanguageSpec { code: "btx", native: "Batak Karo", english: "Batak Karo", flag: "id", rtl: false },
    LanguageSpec { code: "bts", native: "Batak Simalungun", english: "Batak Simalungun", flag: "id", rtl: false },
    LanguageSpec { code: "ceb", native: "Cebuano", english: "Cebuano", flag: "ph", rtl: false },
    LanguageSpec { code: "ilo", native: "Ilokano", english: "Ilocano", flag: "ph", rtl: false },
    LanguageSpec { code: "pam", native: "Kapampangan", english: "Kapampangan", flag: "ph", rtl: false },
    LanguageSpec { code: "pag", native: "Pangasinan", english: "Pangasinan", flag: "ph", rtl: false },
    LanguageSpec { code: "hil", native: "Hiligaynon", english: "Hiligaynon", flag: "ph", rtl: false },
    LanguageSpec { code: "bik", native: "Bikol", english: "Bikol", flag: "ph", rtl: false },
    LanguageSpec { code: "hmn", native: "Hmoob", english: "Hmong", flag: "la", rtl: false },
    LanguageSpec { code: "shn", native: "လိၵ်ႈတႆး", english: "Shan", flag: "mm", rtl: false },
    LanguageSpec { code: "kac", native: "Jinghpaw", english: "Jingpo", flag: "mm", rtl: false },
    LanguageSpec { code: "tet", native: "Tetun", english: "Tetum", flag: "tl", rtl: false },
    LanguageSpec { code: "mi", native: "Te Reo Māori", english: "Maori", flag: "nz", rtl: false },
    LanguageSpec { code: "sm", native: "Gagana Sāmoa", english: "Samoan", flag: "ws", rtl: false },
    LanguageSpec { code: "haw", native: "ʻŌlelo Hawaiʻi", english: "Hawaiian", flag: "us", rtl: false },
    LanguageSpec { code: "fj", native: "Vosa Vakaviti", english: "Fijian", flag: "fj", rtl: false },
    LanguageSpec { code: "to", native: "Lea faka-Tonga", english: "Tongan", flag: "to", rtl: false },
    LanguageSpec { code: "ty", native: "Reo Tahiti", english: "Tahitian", flag: "pf", rtl: false },
    LanguageSpec { code: "tpi", native: "Tok Pisin", english: "Tok Pisin", flag: "pg", rtl: false },
    LanguageSpec { code: "ht", native: "Kreyòl Ayisyen", english: "Haitian Creole", flag: "ht", rtl: false },
    LanguageSpec { code: "pap", native: "Papiamentu", english: "Papiamento", flag: "cw", rtl: false },
    LanguageSpec { code: "jam", native: "Patois", english: "Jamaican Patois", flag: "jm", rtl: false },
    LanguageSpec { code: "qu", native: "Runasimi", english: "Quechua", flag: "pe", rtl: false },
    LanguageSpec { code: "gn", native: "Avañeʼẽ", english: "Guarani", flag: "py", rtl: false },
    LanguageSpec { code: "ay", native: "Aymar aru", english: "Aymara", flag: "bo", rtl: false },
];

/// Core canonical English strings mapped by stable legacy keys.
const CORE_EN: &[(&str, &str)] = &[
    ("nav.messages", "Messages"),
    ("nav.friends", "Friends"),
    ("nav.communities", "Communities"),
    ("nav.search", "Search"),
    ("nav.settings", "Settings"),
    ("nav.profile", "Profile"),
    ("nav.logout", "Log out"),
    ("nav.quick", "Quick switch"),
    ("chat.self", "Note to self"),
    ("chat.search_conv", "Search conversations"),
    ("chat.all", "All"),
    ("chat.unread", "Unread"),
    ("chat.pinned", "Pinned"),
    ("chat.direct", "Direct messages"),
    ("chat.groups", "Groups"),
    ("chat.channels", "Channels"),
    ("chat.bots", "Plugins"),
    ("chat.message_to", "Message {name}"),
    ("chat.e2ee", "End-to-end encrypted"),
    ("chat.typing", "{name} is typing"),
    ("chat.seen", "Seen"),
    ("act.send", "Send"),
    ("act.cancel", "Cancel"),
    ("act.save", "Save"),
    ("act.close", "Close"),
    ("act.delete", "Delete"),
    ("act.join", "Join"),
    ("act.copy_link", "Copy link"),
    ("act.invite", "Invite"),
    ("empty.pick_chat", "Select a conversation"),
    ("settings.language", "Language"),
];

/// Active language index into `LANGUAGES` (index 0 = canonical English `en`).
static LANG: GlobalSignal<usize> = Signal::global(|| 0);

pub fn lang_index(code: &str) -> Option<usize> {
    if let Some(i) = LANGUAGES.iter().position(|l| l.code.eq_ignore_ascii_case(code)) {
        return Some(i);
    }
    // Normalize legacy/regional BCP-47 aliases cleanly (e.g. tl -> fil, jw -> jv, nb -> no, zh-Hans -> zh-CN, vi-VN -> vi)
    let low = code.to_ascii_lowercase();
    let alias = match low.as_str() {
        "tl" | "tl-ph" | "fil-ph" => Some("fil"),
        "jw" | "jv-id" => Some("jv"),
        "nb" | "nb-no" | "nn" | "nn-no" | "no-no" => Some("no"),
        "zh" | "zh-hans" | "zh-sg" => Some("zh-CN"),
        "zh-hant" | "zh-hk" | "zh-mo" => Some("zh-TW"),
        "pt" => Some("pt-BR"),
        "iw" | "he-il" => Some("he"),
        "in" => Some("id"),
        _ => None,
    };
    if let Some(a) = alias {
        if let Some(i) = LANGUAGES.iter().position(|l| l.code == a) {
            return Some(i);
        }
    }
    if let Some((base, _)) = code.split_once('-') {
        return LANGUAGES.iter().position(|l| l.code.eq_ignore_ascii_case(base));
    }
    None
}

pub fn lang_spec(code: &str) -> &'static LanguageSpec {
    lang_index(code).map(|i| &LANGUAGES[i]).unwrap_or(&LANGUAGES[0])
}

pub fn flag_code(code: &str) -> &'static str {
    lang_spec(code).flag
}

/// Return canonical English UI string for a key.
/// On Web, `i18n_runtime.js` is the single DOM owner that translates every canonical English
/// node & attribute into any of the 197 languages without Dioxus virtual-DOM races.
pub fn t(key: &str) -> &'static str {
    let _ = *LANG.read();
    CORE_EN.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).unwrap_or("")
}

/// Return canonical English template with `{name}` substituted.
pub fn tn(key: &str, name: &str) -> String {
    t(key).replace("{name}", name)
}

pub fn current() -> &'static str {
    LANGUAGES.get(*LANG.peek()).map(|l| l.code).unwrap_or("en")
}

/// Determine initial language: persisted `chatw_lang` if set, otherwise canonical English `en`.
pub fn detect_default_language() -> &'static str {
    if let Some(code) = crate::platform::storage_get("chatw_lang") {
        if let Some(idx) = lang_index(&code) {
            return LANGUAGES[idx].code;
        }
    }
    "en"
}

pub fn init_from_storage() {
    let code = detect_default_language();
    set_language(code);
}

pub fn set_language(code: &str) {
    let idx = lang_index(code).unwrap_or(0);
    if *LANG.peek() != idx {
        *LANG.write() = idx;
    }
    let spec = &LANGUAGES[idx];
    let dir = if spec.rtl { "rtl" } else { "ltr" };
    crate::platform::storage_set("chatw_lang", spec.code);
    let code_str = spec.code;
    let _ = dioxus::document::eval(&format!(
        "document.documentElement.setAttribute('lang', {code_str:?}); document.documentElement.setAttribute('dir', {dir:?}); document.documentElement.setAttribute('translate', 'no'); document.documentElement.classList.add('notranslate'); if (window.__chatwI18n && typeof window.__chatwI18n.setLanguage === 'function') {{ window.__chatwI18n.setLanguage({code_str:?}); }}"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn global_language_catalog_is_valid_and_english_first() {
        assert_eq!(LANGUAGES[0].code, "en");
        assert!(LANGUAGES.len() >= 190);
        let mut seen = HashSet::new();
        for lang in LANGUAGES {
            assert!(seen.insert(lang.code), "duplicate code {}", lang.code);
            assert!(!lang.native.trim().is_empty());
            assert!(!lang.english.trim().is_empty());
            assert!(!lang.flag.trim().is_empty());
            assert!(!lang.native.contains("..."));
        }
        assert!(lang_spec("ar").rtl);
        assert!(lang_spec("he").rtl);
        assert!(lang_spec("fa").rtl);
        assert!(lang_spec("ur").rtl);
        assert!(!lang_spec("en").rtl);
        assert!(!lang_spec("vi").rtl);
        assert_eq!(lang_spec("tl").code, "fil");
        assert_eq!(lang_spec("vi-VN").code, "vi");
    }
}
