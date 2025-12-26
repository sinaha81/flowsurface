use crate::{MinTicksize, Price};

use serde::Deserializer;
use serde::de::Error as SerdeError;
use serde_json::Value;

use std::{collections::BTreeMap, sync::Arc};

/// ساختار کمکی برای دی‌سریال‌سازی یک سطح قیمتی در دفتر سفارش
#[derive(Clone, Copy)]
pub struct DeOrder {
    pub price: f32, // قیمت
    pub qty: f32,   // مقدار
}

impl<'de> serde::Deserialize<'de> for DeOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // can be either an array like ["price","qty", ...] or an object with keys "0" and "1"
        let value = Value::deserialize(deserializer).map_err(SerdeError::custom)?;

        let parse_f = |val: &Value| -> Option<f32> {
            match val {
                Value::String(s) => s.parse::<f32>().ok(),
                Value::Number(n) => n.as_f64().map(|x| x as f32),
                _ => None,
            }
        };

        let price = match &value {
            Value::Array(arr) => arr.first().and_then(parse_f),
            Value::Object(map) => map.get("0").and_then(parse_f),
            _ => None,
        }
        .ok_or_else(|| SerdeError::custom("Order price not found or invalid"))?;

        let qty = match &value {
            Value::Array(arr) => arr.get(1).and_then(parse_f),
            Value::Object(map) => map.get("1").and_then(parse_f),
            _ => None,
        }
        .ok_or_else(|| SerdeError::custom("Order qty not found or invalid"))?;

        Ok(DeOrder { price, qty })
    }
}

/// ساختار داخلی برای نمایش یک سفارش با قیمت دقیق
struct Order {
    price: Price,
    qty: f32,
}

/// داده‌های دریافتی مربوط به عمق بازار
pub struct DepthPayload {
    pub last_update_id: u64, // شناسه آخرین بروزرسانی
    pub time: u64,           // زمان بروزرسانی
    pub bids: Vec<DeOrder>,  // لیست قیمت‌های خرید
    pub asks: Vec<DeOrder>,  // لیست قیمت‌های فروش
}

/// انواع بروزرسانی‌های عمق بازار
pub enum DepthUpdate {
    Snapshot(DepthPayload), // تصویر کامل از وضعیت فعلی (Snapshot)
    Diff(DepthPayload),     // تغییرات نسبت به وضعیت قبلی (Delta/Diff)
}

/// ساختار نگهدارنده وضعیت فعلی دفتر سفارش
#[derive(Clone, Default)]
pub struct Depth {
    pub bids: BTreeMap<Price, f32>, // دفتر سفارشات خرید (مرتب شده بر اساس قیمت)
    pub asks: BTreeMap<Price, f32>, // دفتر سفارشات فروش (مرتب شده بر اساس قیمت)
}

impl std::fmt::Debug for Depth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Depth")
            .field("bids", &self.bids.len())
            .field("asks", &self.asks.len())
            .finish()
    }
}

impl Depth {
    fn update(&mut self, diff: &DepthPayload, min_ticksize: MinTicksize) {
        Self::diff_price_levels(&mut self.bids, &diff.bids, min_ticksize);
        Self::diff_price_levels(&mut self.asks, &diff.asks, min_ticksize);
    }

    fn diff_price_levels(
        price_map: &mut BTreeMap<Price, f32>,
        orders: &[DeOrder],
        min_ticksize: MinTicksize,
    ) {
        orders.iter().for_each(|order| {
            let order = Order {
                price: Price::from_f32(order.price).round_to_min_tick(min_ticksize),
                qty: order.qty,
            };

            if order.qty == 0.0 {
                price_map.remove(&order.price);
            } else {
                price_map.insert(order.price, order.qty);
            }
        });
    }

    fn replace_all(&mut self, snapshot: &DepthPayload, min_ticksize: MinTicksize) {
        self.bids = snapshot
            .bids
            .iter()
            .map(|de_order| {
                (
                    Price::from_f32(de_order.price).round_to_min_tick(min_ticksize),
                    de_order.qty,
                )
            })
            .collect::<BTreeMap<Price, f32>>();
        self.asks = snapshot
            .asks
            .iter()
            .map(|de_order| {
                (
                    Price::from_f32(de_order.price).round_to_min_tick(min_ticksize),
                    de_order.qty,
                )
            })
            .collect::<BTreeMap<Price, f32>>();
    }

    pub fn mid_price(&self) -> Option<Price> {
        match (self.asks.first_key_value(), self.bids.last_key_value()) {
            (Some((ask_price, _)), Some((bid_price, _))) => Some((*ask_price + *bid_price) / 2),
            _ => None,
        }
    }
}

/// حافظه موقت محلی برای نگهداری و بروزرسانی عمق بازار یک نماد
#[derive(Default)]
pub struct LocalDepthCache {
    pub last_update_id: u64, // آخرین شناسه بروزرسانی اعمال شده
    pub time: u64,           // زمان آخرین بروزرسانی
    pub depth: Arc<Depth>,   // وضعیت فعلی عمق بازار (به صورت اشتراکی)
}

impl LocalDepthCache {
    pub fn update(&mut self, new_depth: DepthUpdate, min_ticksize: MinTicksize) {
        match new_depth {
            DepthUpdate::Snapshot(snapshot) => {
                self.last_update_id = snapshot.last_update_id;
                self.time = snapshot.time;

                let depth = Arc::make_mut(&mut self.depth);
                depth.replace_all(&snapshot, min_ticksize);
            }
            DepthUpdate::Diff(diff) => {
                self.last_update_id = diff.last_update_id;
                self.time = diff.time;

                let depth = Arc::make_mut(&mut self.depth);
                depth.update(&diff, min_ticksize);
            }
        }
    }
}
