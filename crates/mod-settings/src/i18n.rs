use asrbox_platform_api::AppLanguage;

#[inline]
pub fn tr<'a>(lang: AppLanguage, ru: &'a str, en: &'a str) -> &'a str {
    match lang {
        AppLanguage::Ru => ru,
        AppLanguage::En => en,
    }
}
