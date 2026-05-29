mod genshin;
mod starrail;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum GameKind {
    Genshin,
    StarRail,
}

impl GameKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "genshin" => Some(Self::Genshin),
            "starrail" => Some(Self::StarRail),
            _ => None,
        }
    }

    pub fn process_name(&self) -> &'static str {
        match self {
            Self::Genshin => "YuanShen.exe",
            Self::StarRail => "StarRail.exe",
        }
    }
}

pub struct GameFeatures {
    /// 标题区域比例 (x, y, width, height)
    pub title_region: (f64, f64, f64, f64),
    /// 行记录区域比例 (x, y, width, height, row_height)
    pub row_region: (f64, f64, f64, f64, f64),
    /// 标题关键词
    pub title_keywords: &'static [&'static str],
    /// 物品名校对映射
    pub name_normalizations: &'static [(&'static str, &'static str)],
}

impl GameKind {
    pub fn features(&self) -> GameFeatures {
        match self {
            Self::Genshin => GameFeatures {
                title_region: (0.03, 0.04, 0.5, 0.08),
                row_region: (0.05, 0.12, 0.9, 0.8, 0.07),
                title_keywords: &["历史记录"],
                name_normalizations: &[
                    ("七七·角色", "七七"),
                    ("刻晴·角色", "刻晴"),
                    ("迪卢克·角色", "迪卢克"),
                    ("莫娜·角色", "莫娜"),
                    ("琴·角色", "琴"),
                    ("提纳里·角色", "提纳里"),
                    ("迪希雅·角色", "迪希雅"),
                    ("德赫雅·角色", "迪希雅"),
                ],
            },
            Self::StarRail => GameFeatures {
                title_region: (0.35, 0.02, 0.3, 0.06),
                row_region: (0.05, 0.12, 0.9, 0.8, 0.07),
                title_keywords: &["跃迁记录"],
                name_normalizations: &[
                    ("布洛妮娅·角色", "布洛妮娅"),
                    ("姬子·角色", "姬子"),
                    ("瓦尔特·角色", "瓦尔特"),
                    ("白露·角色", "白露"),
                    ("杰帕德·角色", "杰帕德"),
                    ("彦卿·角色", "彦卿"),
                    ("克拉拉·角色", "克拉拉"),
                ],
            },
        }
    }
}
