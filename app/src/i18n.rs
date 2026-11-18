//! UI language support. The core navigation and chat strings are translated
//! into every language listed below; the remaining screens fall back to
//! Vietnamese. `t()` reads a global signal, so components re-render when the
//! language changes. Right-to-left scripts flip the document direction.

use dioxus::prelude::*;

/// (BCP-47 code, native name, English name)
pub const LANGUAGES: &[(&str, &str, &str)] = &[
    ("vi", "Tiếng Việt", "Vietnamese"),
    ("en", "English", "English"),
    ("zh-CN", "简体中文", "Chinese (Simplified)"),
    ("zh-TW", "繁體中文", "Chinese (Traditional)"),
    ("ja", "日本語", "Japanese"),
    ("ko", "한국어", "Korean"),
    ("th", "ไทย", "Thai"),
    ("id", "Bahasa Indonesia", "Indonesian"),
    ("ms", "Bahasa Melayu", "Malay"),
    ("fil", "Filipino", "Filipino"),
    ("km", "ខ្មែរ", "Khmer"),
    ("lo", "ລາວ", "Lao"),
    ("hi", "हिन्दी", "Hindi"),
    ("bn", "বাংলা", "Bengali"),
    ("ar", "العربية", "Arabic"),
    ("fa", "فارسی", "Persian"),
    ("he", "עברית", "Hebrew"),
    ("ur", "اردو", "Urdu"),
    ("tr", "Türkçe", "Turkish"),
    ("ru", "Русский", "Russian"),
    ("uk", "Українська", "Ukrainian"),
    ("pl", "Polski", "Polish"),
    ("cs", "Čeština", "Czech"),
    ("de", "Deutsch", "German"),
    ("fr", "Français", "French"),
    ("es", "Español", "Spanish"),
    ("pt-BR", "Português (Brasil)", "Portuguese (Brazil)"),
    ("it", "Italiano", "Italian"),
    ("nl", "Nederlands", "Dutch"),
    ("sv", "Svenska", "Swedish"),
    ("ro", "Română", "Romanian"),
    ("hu", "Magyar", "Hungarian"),
    ("el", "Ελληνικά", "Greek"),
];

const RTL: &[&str] = &["ar", "fa", "he", "ur"];

const KEYS: [&str; 31] = [
    "nav.messages", "nav.friends", "nav.communities", "nav.search", "nav.settings", "nav.profile", "nav.logout", "nav.quick",
    "chat.self", "chat.search_conv", "chat.all", "chat.unread", "chat.pinned", "chat.direct", "chat.groups", "chat.channels",
    "chat.bots", "chat.message_to", "chat.e2ee", "chat.typing", "chat.seen", "act.send", "act.cancel", "act.save", "act.close",
    "act.delete", "act.join", "act.copy_link", "act.invite", "empty.pick_chat", "settings.language",
];

type Row = [&'static str; 31];

const TR: &[Row] = &[
    // vi
    ["Tin nhắn", "Bạn bè", "Cộng đồng", "Tìm kiếm", "Cài đặt", "Hồ sơ", "Đăng xuất", "Tìm nhanh", "Chính mình", "Tìm cuộc trò chuyện", "Tất cả", "Chưa đọc", "Đã ghim", "Tin nhắn trực tiếp", "Nhóm", "Kênh", "Bot", "Nhắn tin tới {name}", "Mã hóa đầu cuối", "{name} đang soạn tin", "Đã xem", "Gửi", "Hủy", "Lưu", "Đóng", "Xóa", "Tham gia", "Sao chép liên kết", "Mời", "Chọn một cuộc trò chuyện", "Ngôn ngữ"],
    // en
    ["Messages", "Friends", "Communities", "Search", "Settings", "Profile", "Log out", "Quick switch", "Note to self", "Search conversations", "All", "Unread", "Pinned", "Direct messages", "Groups", "Channels", "Bots", "Message {name}", "End-to-end encrypted", "{name} is typing", "Seen", "Send", "Cancel", "Save", "Close", "Delete", "Join", "Copy link", "Invite", "Select a conversation", "Language"],
    // zh-CN
    ["消息", "好友", "社区", "搜索", "设置", "个人资料", "退出登录", "快速切换", "我自己", "搜索会话", "全部", "未读", "已置顶", "私信", "群组", "频道", "机器人", "发消息给 {name}", "端到端加密", "{name} 正在输入", "已读", "发送", "取消", "保存", "关闭", "删除", "加入", "复制链接", "邀请", "选择一个会话", "语言"],
    // zh-TW
    ["訊息", "好友", "社群", "搜尋", "設定", "個人檔案", "登出", "快速切換", "我自己", "搜尋對話", "全部", "未讀", "已釘選", "私人訊息", "群組", "頻道", "機器人", "傳訊息給 {name}", "端對端加密", "{name} 正在輸入", "已讀", "傳送", "取消", "儲存", "關閉", "刪除", "加入", "複製連結", "邀請", "選擇一個對話", "語言"],
    // ja
    ["メッセージ", "友だち", "コミュニティ", "検索", "設定", "プロフィール", "ログアウト", "クイック切替", "自分", "会話を検索", "すべて", "未読", "ピン留め", "ダイレクトメッセージ", "グループ", "チャンネル", "ボット", "{name} にメッセージ", "エンドツーエンド暗号化", "{name} が入力中", "既読", "送信", "キャンセル", "保存", "閉じる", "削除", "参加", "リンクをコピー", "招待", "会話を選択してください", "言語"],
    // ko
    ["메시지", "친구", "커뮤니티", "검색", "설정", "프로필", "로그아웃", "빠른 전환", "나와의 채팅", "대화 검색", "전체", "읽지 않음", "고정됨", "다이렉트 메시지", "그룹", "채널", "봇", "{name}에게 메시지 보내기", "종단간 암호화", "{name} 님이 입력 중", "읽음", "보내기", "취소", "저장", "닫기", "삭제", "참여", "링크 복사", "초대", "대화를 선택하세요", "언어"],
    // th
    ["ข้อความ", "เพื่อน", "ชุมชน", "ค้นหา", "การตั้งค่า", "โปรไฟล์", "ออกจากระบบ", "สลับด่วน", "ตัวฉันเอง", "ค้นหาการสนทนา", "ทั้งหมด", "ยังไม่อ่าน", "ปักหมุดแล้ว", "ข้อความส่วนตัว", "กลุ่ม", "ช่อง", "บอท", "ส่งข้อความถึง {name}", "เข้ารหัสตั้งแต่ต้นทางถึงปลายทาง", "{name} กำลังพิมพ์", "อ่านแล้ว", "ส่ง", "ยกเลิก", "บันทึก", "ปิด", "ลบ", "เข้าร่วม", "คัดลอกลิงก์", "เชิญ", "เลือกการสนทนา", "ภาษา"],
    // id
    ["Pesan", "Teman", "Komunitas", "Cari", "Pengaturan", "Profil", "Keluar", "Pindah cepat", "Catatan pribadi", "Cari percakapan", "Semua", "Belum dibaca", "Disematkan", "Pesan langsung", "Grup", "Saluran", "Bot", "Kirim pesan ke {name}", "Terenkripsi end-to-end", "{name} sedang mengetik", "Dilihat", "Kirim", "Batal", "Simpan", "Tutup", "Hapus", "Gabung", "Salin tautan", "Undang", "Pilih percakapan", "Bahasa"],
    // ms
    ["Mesej", "Rakan", "Komuniti", "Cari", "Tetapan", "Profil", "Log keluar", "Tukar pantas", "Nota kendiri", "Cari perbualan", "Semua", "Belum dibaca", "Disematkan", "Mesej terus", "Kumpulan", "Saluran", "Bot", "Hantar mesej kepada {name}", "Disulitkan hujung ke hujung", "{name} sedang menaip", "Dilihat", "Hantar", "Batal", "Simpan", "Tutup", "Padam", "Sertai", "Salin pautan", "Jemput", "Pilih perbualan", "Bahasa"],
    // fil
    ["Mga Mensahe", "Mga Kaibigan", "Mga Komunidad", "Maghanap", "Mga Setting", "Profile", "Mag-log out", "Mabilis na paglipat", "Sarili ko", "Maghanap ng usapan", "Lahat", "Hindi pa nababasa", "Naka-pin", "Direktang mensahe", "Mga Grupo", "Mga Channel", "Mga Bot", "Magmensahe kay {name}", "End-to-end encrypted", "Nagta-type si {name}", "Nakita na", "Ipadala", "Kanselahin", "I-save", "Isara", "Burahin", "Sumali", "Kopyahin ang link", "Mag-imbita", "Pumili ng usapan", "Wika"],
    // km
    ["សារ", "មិត្តភក្តិ", "សហគមន៍", "ស្វែងរក", "ការកំណត់", "ប្រវត្តិរូប", "ចាកចេញ", "ប្ដូររហ័ស", "ខ្លួនឯង", "ស្វែងរកការសន្ទនា", "ទាំងអស់", "មិនទាន់អាន", "បានខ្ទាស់", "សារផ្ទាល់", "ក្រុម", "ឆានែល", "បូត", "ផ្ញើសារទៅ {name}", "ការអ៊ិនគ្រីបពីចុងដល់ចុង", "{name} កំពុងវាយ", "បានឃើញ", "ផ្ញើ", "បោះបង់", "រក្សាទុក", "បិទ", "លុប", "ចូលរួម", "ចម្លងតំណ", "អញ្ជើញ", "ជ្រើសរើសការសន្ទនា", "ភាសា"],
    // lo
    ["ຂໍ້ຄວາມ", "ໝູ່ເພື່ອນ", "ຊຸມຊົນ", "ຄົ້ນຫາ", "ການຕັ້ງຄ່າ", "ໂປຣໄຟລ໌", "ອອກຈາກລະບົບ", "ສະຫຼັບດ່ວນ", "ຕົວເອງ", "ຄົ້ນຫາການສົນທະນາ", "ທັງໝົດ", "ຍັງບໍ່ໄດ້ອ່ານ", "ປັກໝຸດແລ້ວ", "ຂໍ້ຄວາມສ່ວນຕົວ", "ກຸ່ມ", "ຊ່ອງ", "ບອດ", "ສົ່ງຂໍ້ຄວາມຫາ {name}", "ເຂົ້າລະຫັດແຕ່ຕົ້ນທາງຫາປາຍທາງ", "{name} ກຳລັງພິມ", "ເຫັນແລ້ວ", "ສົ່ງ", "ຍົກເລີກ", "ບັນທຶກ", "ປິດ", "ລຶບ", "ເຂົ້າຮ່ວມ", "ສຳເນົາລິ້ງ", "ເຊີນ", "ເລືອກການສົນທະນາ", "ພາສາ"],
    // hi
    ["संदेश", "मित्र", "समुदाय", "खोजें", "सेटिंग्स", "प्रोफ़ाइल", "लॉग आउट", "त्वरित स्विच", "स्वयं", "बातचीत खोजें", "सभी", "अपठित", "पिन किए गए", "सीधे संदेश", "समूह", "चैनल", "बॉट", "{name} को संदेश भेजें", "एंड-टू-एंड एन्क्रिप्टेड", "{name} टाइप कर रहे हैं", "देखा गया", "भेजें", "रद्द करें", "सहेजें", "बंद करें", "हटाएँ", "शामिल हों", "लिंक कॉपी करें", "आमंत्रित करें", "एक बातचीत चुनें", "भाषा"],
    // bn
    ["বার্তা", "বন্ধুরা", "কমিউনিটি", "অনুসন্ধান", "সেটিংস", "প্রোফাইল", "লগ আউট", "দ্রুত পরিবর্তন", "নিজে", "কথোপকথন খুঁজুন", "সব", "অপঠিত", "পিন করা", "সরাসরি বার্তা", "গ্রুপ", "চ্যানেল", "বট", "{name}-কে বার্তা পাঠান", "এন্ড-টু-এন্ড এনক্রিপ্টেড", "{name} টাইপ করছেন", "দেখা হয়েছে", "পাঠান", "বাতিল", "সংরক্ষণ", "বন্ধ", "মুছুন", "যোগ দিন", "লিংক কপি করুন", "আমন্ত্রণ", "একটি কথোপকথন বেছে নিন", "ভাষা"],
    // ar
    ["الرسائل", "الأصدقاء", "المجتمعات", "بحث", "الإعدادات", "الملف الشخصي", "تسجيل الخروج", "تبديل سريع", "ملاحظات لنفسي", "البحث في المحادثات", "الكل", "غير مقروءة", "مثبتة", "الرسائل المباشرة", "المجموعات", "القنوات", "الروبوتات", "أرسل رسالة إلى {name}", "مشفّر بين الطرفين", "{name} يكتب الآن", "تمت المشاهدة", "إرسال", "إلغاء", "حفظ", "إغلاق", "حذف", "انضمام", "نسخ الرابط", "دعوة", "اختر محادثة", "اللغة"],
    // fa
    ["پیام‌ها", "دوستان", "انجمن‌ها", "جستجو", "تنظیمات", "نمایه", "خروج", "جابه‌جایی سریع", "یادداشت برای خودم", "جستجوی گفتگوها", "همه", "خوانده‌نشده", "سنجاق‌شده", "پیام‌های مستقیم", "گروه‌ها", "کانال‌ها", "ربات‌ها", "پیام به {name}", "رمزنگاری سرتاسری", "{name} در حال نوشتن است", "دیده شد", "ارسال", "لغو", "ذخیره", "بستن", "حذف", "پیوستن", "کپی پیوند", "دعوت", "یک گفتگو را انتخاب کنید", "زبان"],
    // he
    ["הודעות", "חברים", "קהילות", "חיפוש", "הגדרות", "פרופיל", "התנתקות", "מעבר מהיר", "הערה לעצמי", "חיפוש שיחות", "הכול", "לא נקראו", "מוצמדות", "הודעות ישירות", "קבוצות", "ערוצים", "בוטים", "שליחת הודעה אל {name}", "מוצפן מקצה לקצה", "{name} מקליד/ה", "נראה", "שליחה", "ביטול", "שמירה", "סגירה", "מחיקה", "הצטרפות", "העתקת קישור", "הזמנה", "בחרו שיחה", "שפה"],
    // ur
    ["پیغامات", "دوست", "کمیونٹیز", "تلاش", "ترتیبات", "پروفائل", "لاگ آؤٹ", "فوری تبدیلی", "اپنے لیے نوٹ", "گفتگو تلاش کریں", "سب", "غیر پڑھے", "پن شدہ", "براہ راست پیغامات", "گروپس", "چینلز", "بوٹس", "{name} کو پیغام بھیجیں", "اینڈ ٹو اینڈ انکرپٹڈ", "{name} ٹائپ کر رہے ہیں", "دیکھ لیا", "بھیجیں", "منسوخ", "محفوظ کریں", "بند کریں", "حذف کریں", "شامل ہوں", "لنک کاپی کریں", "مدعو کریں", "ایک گفتگو منتخب کریں", "زبان"],
    // tr
    ["Mesajlar", "Arkadaşlar", "Topluluklar", "Ara", "Ayarlar", "Profil", "Çıkış yap", "Hızlı geçiş", "Kendime not", "Sohbetlerde ara", "Tümü", "Okunmamış", "Sabitlenmiş", "Doğrudan mesajlar", "Gruplar", "Kanallar", "Botlar", "{name} kişisine mesaj gönder", "Uçtan uca şifreli", "{name} yazıyor", "Görüldü", "Gönder", "İptal", "Kaydet", "Kapat", "Sil", "Katıl", "Bağlantıyı kopyala", "Davet et", "Bir sohbet seçin", "Dil"],
    // ru
    ["Сообщения", "Друзья", "Сообщества", "Поиск", "Настройки", "Профиль", "Выйти", "Быстрый переход", "Заметки для себя", "Поиск по чатам", "Все", "Непрочитанные", "Закреплённые", "Личные сообщения", "Группы", "Каналы", "Боты", "Написать {name}", "Сквозное шифрование", "{name} печатает", "Просмотрено", "Отправить", "Отмена", "Сохранить", "Закрыть", "Удалить", "Вступить", "Копировать ссылку", "Пригласить", "Выберите чат", "Язык"],
    // uk
    ["Повідомлення", "Друзі", "Спільноти", "Пошук", "Налаштування", "Профіль", "Вийти", "Швидкий перехід", "Нотатки для себе", "Пошук розмов", "Усі", "Непрочитані", "Закріплені", "Особисті повідомлення", "Групи", "Канали", "Боти", "Написати {name}", "Наскрізне шифрування", "{name} друкує", "Переглянуто", "Надіслати", "Скасувати", "Зберегти", "Закрити", "Видалити", "Приєднатися", "Копіювати посилання", "Запросити", "Виберіть розмову", "Мова"],
    // pl
    ["Wiadomości", "Znajomi", "Społeczności", "Szukaj", "Ustawienia", "Profil", "Wyloguj się", "Szybkie przełączanie", "Notatka dla siebie", "Szukaj rozmów", "Wszystkie", "Nieprzeczytane", "Przypięte", "Wiadomości prywatne", "Grupy", "Kanały", "Boty", "Napisz do {name}", "Szyfrowanie end-to-end", "{name} pisze", "Wyświetlono", "Wyślij", "Anuluj", "Zapisz", "Zamknij", "Usuń", "Dołącz", "Kopiuj link", "Zaproś", "Wybierz rozmowę", "Język"],
    // cs
    ["Zprávy", "Přátelé", "Komunity", "Hledat", "Nastavení", "Profil", "Odhlásit se", "Rychlé přepnutí", "Poznámka pro sebe", "Hledat konverzace", "Vše", "Nepřečtené", "Připnuté", "Přímé zprávy", "Skupiny", "Kanály", "Boti", "Napsat {name}", "Koncové šifrování", "{name} píše", "Zobrazeno", "Odeslat", "Zrušit", "Uložit", "Zavřít", "Smazat", "Připojit se", "Kopírovat odkaz", "Pozvat", "Vyberte konverzaci", "Jazyk"],
    // de
    ["Nachrichten", "Freunde", "Communities", "Suchen", "Einstellungen", "Profil", "Abmelden", "Schnellwechsel", "Notiz an mich", "Unterhaltungen durchsuchen", "Alle", "Ungelesen", "Angeheftet", "Direktnachrichten", "Gruppen", "Kanäle", "Bots", "Nachricht an {name}", "Ende-zu-Ende-verschlüsselt", "{name} schreibt", "Gesehen", "Senden", "Abbrechen", "Speichern", "Schließen", "Löschen", "Beitreten", "Link kopieren", "Einladen", "Wähle eine Unterhaltung", "Sprache"],
    // fr
    ["Messages", "Amis", "Communautés", "Rechercher", "Paramètres", "Profil", "Se déconnecter", "Changement rapide", "Note à moi-même", "Rechercher des conversations", "Tout", "Non lus", "Épinglés", "Messages privés", "Groupes", "Salons", "Bots", "Envoyer un message à {name}", "Chiffré de bout en bout", "{name} écrit", "Vu", "Envoyer", "Annuler", "Enregistrer", "Fermer", "Supprimer", "Rejoindre", "Copier le lien", "Inviter", "Sélectionnez une conversation", "Langue"],
    // es
    ["Mensajes", "Amigos", "Comunidades", "Buscar", "Ajustes", "Perfil", "Cerrar sesión", "Cambio rápido", "Nota personal", "Buscar conversaciones", "Todo", "No leídos", "Fijados", "Mensajes directos", "Grupos", "Canales", "Bots", "Enviar mensaje a {name}", "Cifrado de extremo a extremo", "{name} está escribiendo", "Visto", "Enviar", "Cancelar", "Guardar", "Cerrar", "Eliminar", "Unirse", "Copiar enlace", "Invitar", "Selecciona una conversación", "Idioma"],
    // pt-BR
    ["Mensagens", "Amigos", "Comunidades", "Pesquisar", "Configurações", "Perfil", "Sair", "Troca rápida", "Nota para mim", "Pesquisar conversas", "Todas", "Não lidas", "Fixadas", "Mensagens diretas", "Grupos", "Canais", "Bots", "Enviar mensagem para {name}", "Criptografia de ponta a ponta", "{name} está digitando", "Visto", "Enviar", "Cancelar", "Salvar", "Fechar", "Excluir", "Entrar", "Copiar link", "Convidar", "Selecione uma conversa", "Idioma"],
    // it
    ["Messaggi", "Amici", "Community", "Cerca", "Impostazioni", "Profilo", "Esci", "Cambio rapido", "Nota personale", "Cerca conversazioni", "Tutti", "Non letti", "Fissati", "Messaggi diretti", "Gruppi", "Canali", "Bot", "Invia un messaggio a {name}", "Crittografia end-to-end", "{name} sta scrivendo", "Visualizzato", "Invia", "Annulla", "Salva", "Chiudi", "Elimina", "Unisciti", "Copia link", "Invita", "Seleziona una conversazione", "Lingua"],
    // nl
    ["Berichten", "Vrienden", "Community's", "Zoeken", "Instellingen", "Profiel", "Uitloggen", "Snel wisselen", "Notitie aan mezelf", "Gesprekken zoeken", "Alles", "Ongelezen", "Vastgezet", "Privéberichten", "Groepen", "Kanalen", "Bots", "Bericht aan {name}", "End-to-end versleuteld", "{name} typt", "Gezien", "Verzenden", "Annuleren", "Opslaan", "Sluiten", "Verwijderen", "Deelnemen", "Link kopiëren", "Uitnodigen", "Kies een gesprek", "Taal"],
    // sv
    ["Meddelanden", "Vänner", "Gemenskaper", "Sök", "Inställningar", "Profil", "Logga ut", "Snabbväxling", "Anteckning till mig själv", "Sök konversationer", "Alla", "Olästa", "Fästa", "Direktmeddelanden", "Grupper", "Kanaler", "Bottar", "Skicka meddelande till {name}", "Totalsträckskrypterad", "{name} skriver", "Sett", "Skicka", "Avbryt", "Spara", "Stäng", "Ta bort", "Gå med", "Kopiera länk", "Bjud in", "Välj en konversation", "Språk"],
    // ro
    ["Mesaje", "Prieteni", "Comunități", "Căutare", "Setări", "Profil", "Deconectare", "Comutare rapidă", "Notă pentru mine", "Caută conversații", "Toate", "Necitite", "Fixate", "Mesaje directe", "Grupuri", "Canale", "Boți", "Trimite un mesaj lui {name}", "Criptat integral", "{name} scrie", "Văzut", "Trimite", "Anulează", "Salvează", "Închide", "Șterge", "Alătură-te", "Copiază linkul", "Invită", "Alege o conversație", "Limbă"],
    // hu
    ["Üzenetek", "Barátok", "Közösségek", "Keresés", "Beállítások", "Profil", "Kijelentkezés", "Gyorsváltás", "Jegyzet magamnak", "Beszélgetések keresése", "Összes", "Olvasatlan", "Kitűzött", "Közvetlen üzenetek", "Csoportok", "Csatornák", "Botok", "Üzenet neki: {name}", "Végpontok közötti titkosítás", "{name} éppen ír", "Látta", "Küldés", "Mégse", "Mentés", "Bezárás", "Törlés", "Csatlakozás", "Link másolása", "Meghívás", "Válassz egy beszélgetést", "Nyelv"],
    // el
    ["Μηνύματα", "Φίλοι", "Κοινότητες", "Αναζήτηση", "Ρυθμίσεις", "Προφίλ", "Αποσύνδεση", "Γρήγορη εναλλαγή", "Σημείωση για μένα", "Αναζήτηση συνομιλιών", "Όλα", "Μη αναγνωσμένα", "Καρφιτσωμένα", "Άμεσα μηνύματα", "Ομάδες", "Κανάλια", "Bots", "Μήνυμα προς {name}", "Κρυπτογράφηση από άκρο σε άκρο", "{name} πληκτρολογεί", "Προβλήθηκε", "Αποστολή", "Ακύρωση", "Αποθήκευση", "Κλείσιμο", "Διαγραφή", "Συμμετοχή", "Αντιγραφή συνδέσμου", "Πρόσκληση", "Επιλέξτε μια συνομιλία", "Γλώσσα"],
];

/// Index into `LANGUAGES` / `TR` of the active language.
static LANG: GlobalSignal<usize> = Signal::global(|| 0);

pub fn lang_index(code: &str) -> Option<usize> {
    LANGUAGES.iter().position(|(c, _, _)| *c == code)
}

/// Translate a core UI key; unknown keys return the key itself.
pub fn t(key: &str) -> &'static str {
    let li = *LANG.read();
    let Some(ki) = KEYS.iter().position(|k| *k == key) else {
        return "";
    };
    TR.get(li).map(|row| row[ki]).unwrap_or(TR[0][ki])
}

/// Translate a key containing `{name}`.
pub fn tn(key: &str, name: &str) -> String {
    t(key).replace("{name}", name)
}

pub fn current() -> &'static str {
    LANGUAGES.get(*LANG.peek()).map(|l| l.0).unwrap_or("vi")
}

pub fn set_language(code: &str) {
    let idx = lang_index(code).unwrap_or(0);
    if *LANG.peek() != idx {
        *LANG.write() = idx;
    }
    let code = LANGUAGES[idx].0;
    let dir = if RTL.contains(&code) { "rtl" } else { "ltr" };
    let _ = dioxus::document::eval(&format!(
        "document.documentElement.setAttribute('lang', {code:?}); document.documentElement.setAttribute('dir', {dir:?});"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_a_row_and_names() {
        assert_eq!(LANGUAGES.len(), TR.len());
        for row in TR {
            for (i, cell) in row.iter().enumerate() {
                assert!(!cell.trim().is_empty(), "empty translation for {}", KEYS[i]);
                assert!(!cell.contains("..."), "placeholder ellipsis in {}", KEYS[i]);
            }
            assert!(row[17].contains("{name}") && row[19].contains("{name}"));
        }
    }
}
