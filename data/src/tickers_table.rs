use exchange::{
    Ticker, TickerStats,
    adapter::{Exchange, ExchangeInclusive, MarketKind},
};
use serde::{Deserialize, Serialize};

/// تنظیمات مربوط به جدول نمادهای معاملاتی
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub favorited_tickers: Vec<Ticker>,      // لیست نمادهای مورد علاقه
    pub show_favorites: bool,                // نمایش فقط مورد علاقه‌ها
    pub selected_sort_option: SortOptions,   // گزینه مرتب‌سازی انتخاب شده
    pub selected_exchanges: Vec<ExchangeInclusive>, // صرافی‌های انتخاب شده
    pub selected_markets: Vec<MarketKind>,   // بازارهای انتخاب شده (Spot, Futures, ...)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            favorited_tickers: vec![],
            show_favorites: false,
            selected_sort_option: SortOptions::VolumeDesc,
            selected_exchanges: ExchangeInclusive::ALL.to_vec(),
            selected_markets: MarketKind::ALL.into_iter().collect(),
        }
    }
}

/// گزینه‌های مرتب‌سازی جدول
#[derive(Default, Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum SortOptions {
    #[default]
    VolumeAsc,  // حجم صعودی
    VolumeDesc, // حجم نزولی
    ChangeAsc,  // تغییرات صعودی
    ChangeDesc, // تغییرات نزولی
}

/// جهت تغییر قیمت
#[derive(Clone, Debug, PartialEq)]
pub enum PriceChangeDirection {
    Increased, // افزایش یافته
    Decreased, // کاهش یافته
    Unchanged, // بدون تغییر
}

/// داده‌های خام یک ردیف در جدول نمادها
#[derive(Clone, Copy)]
pub struct TickerRowData {
    pub exchange: Exchange,                 // صرافی
    pub ticker: Ticker,                     // نماد
    pub stats: TickerStats,                 // آمار فعلی
    pub previous_stats: Option<TickerStats>, // آمار قبلی (برای مقایسه)
    pub is_favorited: bool,                 // آیا در لیست علاقه‌مندی‌هاست؟
}

/// داده‌های آماده برای نمایش در رابط کاربری
#[derive(Clone)]
pub struct TickerDisplayData {
    pub display_ticker: String,               // نام نمایشی نماد
    pub daily_change_pct: String,             // درصد تغییرات روزانه
    pub volume_display: String,               // حجم نمایشی (اختصاری)
    pub mark_price_display: String,           // قیمت لحظه‌ای نمایشی
    pub price_unchanged_part: String,         // بخش بدون تغییر قیمت (برای هایلایت)
    pub price_changed_part: String,           // بخش تغییر یافته قیمت
    pub price_change_direction: PriceChangeDirection, // جهت تغییر قیمت
    pub card_color_alpha: f32,                // شفافیت رنگ کارت بر اساس تغییرات
}

/// محاسبه داده‌های نمایشی بر اساس آمار فعلی و قیمت قبلی
pub fn compute_display_data(
    ticker: &Ticker,
    stats: &TickerStats,
    previous_price: Option<f32>,
) -> TickerDisplayData {
    let (display_ticker, _market) = ticker.display_symbol_and_type();

    let current_price = stats.mark_price;
    let (price_unchanged_part, price_changed_part, price_change_direction) =
        if let Some(prev_price) = previous_price {
            split_price_changes(prev_price, current_price)
        } else {
            (
                current_price.to_string(),
                String::new(),
                PriceChangeDirection::Unchanged,
            )
        };

    TickerDisplayData {
        display_ticker,
        daily_change_pct: super::util::pct_change(stats.daily_price_chg),
        volume_display: super::util::currency_abbr(stats.daily_volume),
        mark_price_display: stats.mark_price.to_string(),
        price_unchanged_part,
        price_changed_part,
        price_change_direction,
        card_color_alpha: { (stats.daily_price_chg / 8.0).clamp(-1.0, 1.0) },
    }
}

/// تشخیص بخش‌های تغییر یافته و ثابت قیمت برای هایلایت کردن در UI
fn split_price_changes(
    previous_price: f32,
    current_price: f32,
) -> (String, String, PriceChangeDirection) {
    if previous_price == current_price {
        return (
            current_price.to_string(),
            String::new(),
            PriceChangeDirection::Unchanged,
        );
    }

    let prev_str = previous_price.to_string();
    let curr_str = current_price.to_string();

    let direction = if current_price > previous_price {
        PriceChangeDirection::Increased
    } else {
        PriceChangeDirection::Decreased
    };

    let mut split_index = 0;
    let prev_chars: Vec<char> = prev_str.chars().collect();
    let curr_chars: Vec<char> = curr_str.chars().collect();

    // پیدا کردن اولین کاراکتری که تغییر کرده است
    for (i, &curr_char) in curr_chars.iter().enumerate() {
        if i >= prev_chars.len() || prev_chars[i] != curr_char {
            split_index = i;
            break;
        }
    }

    if split_index == 0 && curr_chars.len() != prev_chars.len() {
        split_index = prev_chars.len().min(curr_chars.len());
    }

    let unchanged_part = curr_str[..split_index].to_string();
    let changed_part = curr_str[split_index..].to_string();

    (unchanged_part, changed_part, direction)
}
