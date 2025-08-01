use crate::id::DimensionRange;
use crate::id::contain::Containment::{self, Full, Partial};
use crate::{id::SpaceTimeId, set::SpaceTimeIdSet};

impl SpaceTimeIdSet {
    /// Inserts a `SpaceTimeId` into the `SpaceTimeIdSet`, avoiding redundant or overlapping entries.
    ///
    /// - If the set is empty, the ID is added directly.
    /// - If an existing ID fully contains the new one, nothing is added.
    /// - If there is a partial overlap, only the non-overlapping portion is inserted.
    /// - If no overlap, it is inserted as-is.
    ///
    /// # Arguments
    ///
    /// * `other` - The `SpaceTimeId` to insert.
    pub fn insert(&mut self, other: SpaceTimeId) {
        //最初はinnnerに突っ込んでよし
        if self.is_empty() {
            //ZとIに関して粒度の最適化を実施
            self.inner.push(Self::optimal_z(Self::optimal_i(other)));

            return;
        }

        let mut should_insert = true;

        for stid in &self.inner {
            match stid.containment_relation(&other) {
                Full => {
                    return;
                }
                Partial(overlapping) => {
                    let mut overlap_set = SpaceTimeIdSet::new();
                    overlap_set.insert(overlapping);

                    let tmp2 = SpaceTimeIdSet::from(other.clone());
                    let difference = (!overlap_set) & tmp2;

                    let result = self.clone() | difference;
                    self.inner = result.inner;
                    should_insert = false;
                    break;
                }
                Containment::None => {
                    continue;
                }
            }
        }

        if should_insert {
            Self::optimal_push(self, other);
        }
    }

    fn scale_range_for_z_u64(range: DimensionRange<u64>, delta_z: u16) -> DimensionRange<u64> {
        let scale = u64::from(2_u16.pow(delta_z as u32));
        match range {
            DimensionRange::Single(_) => {
                panic!("このパターンは上位で除外されているはず");
            }
            DimensionRange::LimitRange(s, e) => DimensionRange::LimitRange(s / scale, e / scale),
            DimensionRange::AfterUnLimitRange(s) => DimensionRange::AfterUnLimitRange(s / scale),
            DimensionRange::BeforeUnLimitRange(e) => DimensionRange::BeforeUnLimitRange(e / scale),
            DimensionRange::Any => DimensionRange::Any,
        }
    }

    fn scale_range_for_z_i64(range: DimensionRange<i64>, delta_z: u16) -> DimensionRange<i64> {
        let scale = 2_i64.pow(delta_z as u32);
        match range {
            DimensionRange::Single(_) => {
                panic!("このパターンは上位で除外されているはず");
            }
            DimensionRange::LimitRange(s, e) => DimensionRange::LimitRange(s / scale, e / scale),
            DimensionRange::AfterUnLimitRange(s) => DimensionRange::AfterUnLimitRange(s / scale),
            DimensionRange::BeforeUnLimitRange(e) => DimensionRange::BeforeUnLimitRange(e / scale),
            DimensionRange::Any => DimensionRange::Any,
        }
    }

    // Zに関する最適化を行う関数
    fn optimal_z(other: SpaceTimeId) -> SpaceTimeId {
        let x = match Self::optimal_xy_max_z(other.x(), other.z()) {
            Some(v) => v,
            None => return other,
        };

        let y = match Self::optimal_xy_max_z(other.y(), other.z()) {
            Some(v) => v,
            None => return other,
        };
        let f = match Self::optimal_f_max_z(other.f(), other.z()) {
            Some(v) => v,
            None => return other,
        };

        let max_z = x.max(y).max(f);
        let delta_z = other.z() - max_z;

        let new_x = Self::scale_range_for_z_u64(other.x(), delta_z);
        let new_y = Self::scale_range_for_z_u64(other.y(), delta_z);
        let new_f = Self::scale_range_for_z_i64(other.f(), delta_z);

        SpaceTimeId::new(max_z, new_f, new_x, new_y, other.i(), other.t()).unwrap()
    }

    /// その次元範囲に対する最適ZoomLevelを計算する（z: u16, 戻り値: Option<u16>）
    /// 最適値が今と同じ場合はNoneを返す
    fn optimal_max_z_for_range<T, F>(range: DimensionRange<T>, z: u16, to_u64: F) -> Option<u16>
    where
        F: Fn(T) -> u64,
    {
        let result = match range {
            DimensionRange::Single(_) => return None,
            DimensionRange::LimitRange(s, e) => {
                let len = to_u64(e).saturating_sub(to_u64(s)).saturating_add(1);
                z.saturating_sub(Self::count_trailing_zeros(len))
            }
            DimensionRange::BeforeUnLimitRange(e) => {
                let len = to_u64(e).saturating_add(1);
                z.saturating_sub(Self::count_trailing_zeros(len))
            }
            DimensionRange::AfterUnLimitRange(s) => {
                let max = 1u64 << z;
                let len = max.saturating_sub(to_u64(s));
                z.saturating_sub(Self::count_trailing_zeros(len))
            }
            DimensionRange::Any => 0,
        };

        if result == z { None } else { Some(result) }
    }

    /// XY（u64）次元用
    fn optimal_xy_max_z(range: DimensionRange<u64>, z: u16) -> Option<u16> {
        Self::optimal_max_z_for_range(range, z, |x| x)
    }

    /// F（i64）次元用
    fn optimal_f_max_z(range: DimensionRange<i64>, z: u16) -> Option<u16> {
        Self::optimal_max_z_for_range(range, z, |x| x as u64)
    }

    /// その数が2で何回割れるかを返す（戻り値: u16）
    fn count_trailing_zeros(mut n: u64) -> u16 {
        let mut count = 0u16;
        while n % 2 == 0 && n != 0 {
            n /= 2;
            count += 1;
        }
        count
    }

    //Iに関する最適化を行う関数
    pub fn optimal_i(other: SpaceTimeId) -> SpaceTimeId {
        let start;
        let end;

        match other.t() {
            DimensionRange::LimitRange(s, e) => {
                start = s;
                end = e + 1
            }
            DimensionRange::BeforeUnLimitRange(e) => {
                start = 0;
                end = e + 1
            }
            DimensionRange::AfterUnLimitRange(_) => return other,
            DimensionRange::Single(s) => {
                start = s;
                end = s + 1
            }
            DimensionRange::Any => return other,
        }
        let start = other.i() * start;
        let end = other.i() * end;

        let gcd = SpaceTimeId::gcd(start, end);

        if gcd == other.i() {
            return other;
        } else {
            return SpaceTimeId::new(
                other.z(),
                other.f(),
                other.x(),
                other.y(),
                gcd,
                DimensionRange::LimitRange(start / gcd, end / gcd),
            )
            .unwrap();
        }
    }

    //連続最適化を行う関数
    pub fn optimal_push(&mut self, other: SpaceTimeId) {
        for stid in &mut self.inner {
            // Zoom level and interval must match to allow merging
            if stid.z() != other.z() || stid.i() != other.i() {
                continue;
            }

            let matches = [
                stid.x() == other.x(),
                stid.y() == other.y(),
                stid.f() == other.f(),
                stid.t() == other.t(),
            ];

            let match_count = matches.iter().filter(|&&m| m).count();

            if match_count != 3 {
                continue;
            }

            let merged = if !matches[0] {
                Self::to_continuous_xy(stid.x(), other.x())
                    .ok()
                    .flatten()
                    .map(|merged_x| {
                        SpaceTimeId::new(stid.z(), stid.f(), merged_x, stid.y(), stid.i(), stid.t())
                    })
            } else if !matches[1] {
                Self::to_continuous_xy(stid.y(), other.y())
                    .ok()
                    .flatten()
                    .map(|merged_y| {
                        SpaceTimeId::new(stid.z(), stid.f(), stid.x(), merged_y, stid.i(), stid.t())
                    })
            } else if !matches[2] {
                Self::to_continuous_f(stid.f(), other.f())
                    .ok()
                    .flatten()
                    .map(|merged_f| {
                        SpaceTimeId::new(stid.z(), merged_f, stid.x(), stid.y(), stid.i(), stid.t())
                    })
            } else if !matches[3] {
                Self::to_continuous_t(stid.t(), other.t())
                    .ok()
                    .flatten()
                    .map(|merged_t| {
                        SpaceTimeId::new(stid.z(), stid.f(), stid.x(), stid.y(), stid.i(), merged_t)
                    })
            } else {
                None
            };

            if let Some(Ok(new_stid)) = merged {
                *stid = Self::optimal_z(Self::optimal_i(new_stid));
                return; // merged successfully
            }
        }
        //ZとIに関して粒度の最適化を実施
        self.inner.push(Self::optimal_z(Self::optimal_i(other)));
    }

    fn to_continuous_xy(
        target: DimensionRange<u64>,
        other: DimensionRange<u64>,
    ) -> Result<Option<DimensionRange<u64>>, String> {
        Self::to_continuous_range(target, other)
    }

    fn to_continuous_f(
        target: DimensionRange<i64>,
        other: DimensionRange<i64>,
    ) -> Result<Option<DimensionRange<i64>>, String> {
        Self::to_continuous_range(target, other)
    }

    fn to_continuous_t(
        target: DimensionRange<u32>,
        other: DimensionRange<u32>,
    ) -> Result<Option<DimensionRange<u32>>, String> {
        Self::to_continuous_range(target, other)
    }

    fn to_continuous_range<T>(
        target: DimensionRange<T>,
        other: DimensionRange<T>,
    ) -> Result<Option<DimensionRange<T>>, String>
    where
        T: Copy
            + PartialOrd
            + Eq
            + std::fmt::Debug
            + std::ops::Add<Output = T>
            + std::ops::Sub<Output = T>
            + From<u8>,
    {
        match target {
            DimensionRange::Single(v) => match other {
                DimensionRange::Single(s) => {
                    if v + T::from(1) == s {
                        Ok(Some(DimensionRange::LimitRange(v, s)))
                    } else if s + T::from(1) == v {
                        Ok(Some(DimensionRange::LimitRange(s, v)))
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::LimitRange(s, e) => {
                    if s > v {
                        if s - T::from(1) == v {
                            Ok(Some(DimensionRange::LimitRange(v, e)))
                        } else {
                            Ok(None)
                        }
                    } else if e < v {
                        if e + T::from(1) == v {
                            Ok(Some(DimensionRange::LimitRange(s, v)))
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::AfterUnLimitRange(s) => {
                    if s - T::from(1) == v {
                        Ok(Some(DimensionRange::AfterUnLimitRange(v)))
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::BeforeUnLimitRange(e) => {
                    if e + T::from(1) == v {
                        Ok(Some(DimensionRange::BeforeUnLimitRange(v)))
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::Any => Err("重なりがある値が入力されました".to_string()),
            },
            DimensionRange::LimitRange(vs, ve) => match other {
                DimensionRange::Single(_) => Self::to_continuous_range(other, target),
                DimensionRange::LimitRange(s, e) => {
                    if ve + T::from(1) == s {
                        Ok(Some(DimensionRange::LimitRange(vs, e)))
                    } else if e + T::from(1) == vs {
                        Ok(Some(DimensionRange::LimitRange(s, ve)))
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::AfterUnLimitRange(s) => {
                    if ve + T::from(1) == s {
                        Ok(Some(DimensionRange::AfterUnLimitRange(vs)))
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::BeforeUnLimitRange(e) => {
                    if e + T::from(1) == vs {
                        Ok(Some(DimensionRange::BeforeUnLimitRange(ve)))
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::Any => Err("重なりがある値が入力されました".to_string()),
            },
            DimensionRange::AfterUnLimitRange(vs) => match other {
                DimensionRange::BeforeUnLimitRange(e) => {
                    if vs + T::from(1) == e {
                        Ok(Some(DimensionRange::Any))
                    } else {
                        Ok(None)
                    }
                }
                DimensionRange::AfterUnLimitRange(_) => {
                    Err("重なりがある値が入力されました".to_string())
                }
                DimensionRange::Any => Err("重なりがある値が入力されました".to_string()),
                _ => Self::to_continuous_range(other, target),
            },
            DimensionRange::BeforeUnLimitRange(_) => match other {
                DimensionRange::BeforeUnLimitRange(_) => {
                    Err("重なりがある値が入力されました".to_string())
                }
                DimensionRange::Any => Err("重なりがある値が入力されました".to_string()),
                _ => Self::to_continuous_range(other, target),
            },
            DimensionRange::Any => Err("重なりがある値が入力されました".to_string()),
        }
    }
}
