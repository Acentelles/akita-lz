//! L3 recursion experiment (feature `l3-compression-weights`, default off).
//!
//! The canonical builder pushes one event per (gadget bit, source row,
//! coefficient block) for every recomposition, and one event per column for
//! every compression row. Each such family is an arithmetic progression of
//! aligned events whose scalars are a common row factor times a shared table
//! (the lifted gadget powers, or one evaluated matrix's columns). This module
//! records each family as one [`CompressionEventGroup`] and evaluates the
//! multilinear extension of its events by factor:
//!
//! `sum_j sigma * y_j * L * eq(R, h0 + j T) = sigma * L * S`, where `L` is the
//! shared alpha low factor and `S` is computed either by dyadic blocks (when
//! `T` is a power of two; geometric tables close each block in a product of
//! `log` factors) or by a tight loop over the canonical equality window.
//!
//! Every factorization splits disjoint digit positions of one index, so it is
//! exact at arbitrary field points (it never multiplies the extensions of two
//! tables over the same variables). A family that does not satisfy every
//! precondition is pushed by the verbatim canonical loop, so errors and
//! accepted inputs are unchanged. Prover-side materialization expands groups
//! back into their canonical events. The derivation and proofs are in the
//! lever's `FACTORIZATION.md`.

use super::*;
#[allow(unused_imports)]
use jolt_field::solinas::parallel::*;

/// Shared value table of one or more groups.
#[derive(Clone, Debug)]
pub(super) struct GroupTable<E: Field> {
    values: Vec<E>,
    /// `beta^(2^i)` for `i <= floor(log2(len))` when the table is verified
    /// geometric (`values[0] == 1`, `values[j] == values[j - 1] * beta`).
    ratio_squares: Option<Vec<E>>,
    /// Every entry is nonzero.
    all_nonzero: bool,
}

impl<E: Field> GroupTable<E> {
    fn new(values: Vec<E>, check_geometric: bool) -> Self {
        let all_nonzero = values.iter().all(|value| !value.is_zero());
        let ratio_squares = if check_geometric {
            Self::geometric_squares(&values)
        } else {
            None
        };
        Self {
            values,
            ratio_squares,
            all_nonzero,
        }
    }

    fn geometric_squares(values: &[E]) -> Option<Vec<E>> {
        let first = *values.first()?;
        if first != E::one() {
            return None;
        }
        let beta = values.get(1).copied().unwrap_or_else(E::one);
        let mut previous = first;
        for &value in values.iter().skip(1) {
            if value != previous * beta {
                return None;
            }
            previous = value;
        }
        let levels = usize::BITS as usize - values.len().leading_zeros() as usize;
        let mut squares = Vec::with_capacity(levels.max(1));
        let mut square = beta;
        for _ in 0..levels.max(1) {
            squares.push(square);
            let next = square * square;
            square = next;
        }
        Some(squares)
    }
}

/// One arithmetic progression of aligned compression events:
/// `(physical_start + j * stride, coefficient_count, alpha_exponent_start,
/// scalar * table[j])` for `j < len`.
#[derive(Clone, Debug)]
pub(super) struct CompressionEventGroup<E: Field> {
    physical_start: usize,
    stride: usize,
    len: usize,
    coefficient_count: usize,
    alpha_exponent_start: usize,
    scalar: E,
    table: usize,
}

/// Group storage carried by [`CompressionRelationWeights`] under the feature.
#[derive(Clone, Debug)]
pub(super) struct Groups<E: Field> {
    groups: Vec<CompressionEventGroup<E>>,
    tables: Vec<GroupTable<E>>,
    /// `(gadget levels, table index)`.
    gadget_tables: Vec<(usize, usize)>,
    /// `(evaluated matrix index, table index)`.
    column_tables: Vec<(usize, usize)>,
}

#[allow(clippy::derivable_impls)]
impl<E: Field> Default for Groups<E> {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            tables: Vec::new(),
            gadget_tables: Vec::new(),
            column_tables: Vec::new(),
        }
    }
}

/// Equality factor of `value` over `coordinates` (little-endian digits); zero
/// when `value` is outside the coordinates' domain, as `eq_eval_at_index`.
fn eq_bits<E: Field>(coordinates: &[E], value: usize) -> E {
    if coordinates.len() < usize::BITS as usize && value >> coordinates.len() != 0 {
        return E::zero();
    }
    coordinates
        .iter()
        .enumerate()
        .fold(E::one(), |acc, (bit, &coordinate)| {
            if bit < usize::BITS as usize && (value >> bit) & 1 == 1 {
                acc * coordinate
            } else {
                acc * (E::one() - coordinate)
            }
        })
}

impl<E: Field> CompressionRelationWeights<E> {
    /// Canonical admission test of one event (the checks of `push`), without
    /// recording it.
    fn admits_event(
        &self,
        physical_start: usize,
        coefficient_count: usize,
        alpha_exponent_start: usize,
    ) -> bool {
        let (Some(physical_end), Some(alpha_end)) = (
            physical_start.checked_add(coefficient_count),
            alpha_exponent_start.checked_add(coefficient_count),
        ) else {
            return false;
        };
        coefficient_count != 0
            && coefficient_count.is_power_of_two()
            && physical_start.is_multiple_of(self.coefficient_block_len)
            && coefficient_count.is_multiple_of(self.coefficient_block_len)
            && alpha_exponent_start.is_multiple_of(self.coefficient_block_len)
            && physical_end <= self.physical_field_len
            && alpha_end <= self.alpha_powers.len()
    }

    /// Whether every element `j < len` of a progression would be admitted by
    /// `push` and takes the aligned evaluation path. The admission checks are
    /// affine or monotone in `j`, so the first two and the last elements
    /// decide them.
    fn admits_progression(
        &self,
        physical_start: usize,
        stride: usize,
        len: usize,
        coefficient_count: usize,
        alpha_exponent_start: usize,
    ) -> bool {
        let Some(last_offset) = len.checked_sub(1).and_then(|last| last.checked_mul(stride)) else {
            return len == 0;
        };
        let Some(last_start) = physical_start.checked_add(last_offset) else {
            return false;
        };
        let second_ok = len < 2
            || physical_start.checked_add(stride).is_some_and(|second| {
                self.admits_event(second, coefficient_count, alpha_exponent_start)
            });
        coefficient_count.is_power_of_two()
            && physical_start.is_multiple_of(coefficient_count)
            && (len < 2 || stride.is_multiple_of(coefficient_count))
            && self.admits_event(physical_start, coefficient_count, alpha_exponent_start)
            && second_ok
            && self.admits_event(last_start, coefficient_count, alpha_exponent_start)
    }

    fn gadget_table<F>(&mut self, levels: usize) -> usize
    where
        F: Field + CanonicalEncoding,
        E: ExtField<F>,
    {
        if let Some(&(_, index)) = self
            .groups
            .gadget_tables
            .iter()
            .find(|(cached, _)| *cached == levels)
        {
            return index;
        }
        let values = gadget_row_scalars::<F>(levels, 1)
            .into_iter()
            .map(<E as ExtField<F>>::lift_base)
            .collect::<Vec<_>>();
        self.groups.tables.push(GroupTable::new(values, true));
        let index = self.groups.tables.len() - 1;
        self.groups.gadget_tables.push((levels, index));
        index
    }

    /// Record the column events of one compression row: element `j` is
    /// `(span_start + j * ring_dimension, ring_dimension, 0, row_weight *
    /// columns[j])`. Falls back to the canonical loop when any precondition
    /// fails.
    pub(super) fn push_column_group(
        &mut self,
        span_start: usize,
        ring_dimension: usize,
        input_width: usize,
        row_weight: E,
        matrix_index: usize,
        columns: &[E],
    ) -> Result<(), AkitaError> {
        let len = columns
            .get(..input_width)
            .and_then(|live| live.iter().rposition(|value| !value.is_zero()))
            .map_or(0, |last| last + 1);
        let grouped = columns.len() >= input_width
            && (row_weight.is_zero()
                || len == 0
                || self.admits_progression(span_start, ring_dimension, len, ring_dimension, 0));
        if !grouped {
            return self.push_columns_canonical(
                span_start,
                ring_dimension,
                input_width,
                row_weight,
                columns,
            );
        }
        if row_weight.is_zero() || len == 0 {
            // Every canonical scalar is zero, so `push` records nothing.
            return Ok(());
        }
        let table = if let Some(&(_, index)) = self
            .groups
            .column_tables
            .iter()
            .find(|(cached, _)| *cached == matrix_index)
        {
            index
        } else {
            self.groups.tables.push(GroupTable::new(
                columns.get(..input_width).unwrap_or(columns).to_vec(),
                false,
            ));
            let index = self.groups.tables.len() - 1;
            self.groups.column_tables.push((matrix_index, index));
            index
        };
        self.groups.groups.push(CompressionEventGroup {
            physical_start: span_start,
            stride: ring_dimension,
            len,
            coefficient_count: ring_dimension,
            alpha_exponent_start: 0,
            scalar: row_weight,
            table,
        });
        Ok(())
    }

    /// The canonical column loop of `build_compression_relation_weights`,
    /// verbatim.
    fn push_columns_canonical(
        &mut self,
        span_start: usize,
        ring_dimension: usize,
        input_width: usize,
        row_weight: E,
        columns: &[E],
    ) -> Result<(), AkitaError> {
        for column in 0..input_width {
            let start = column * ring_dimension;
            self.push(
                span_start + start,
                ring_dimension,
                0,
                row_weight
                    * columns.get(column).copied().ok_or_else(|| {
                        AkitaError::Internal(
                            "compression column exceeds the evaluated column table".into(),
                        )
                    })?,
            )?;
        }
        Ok(())
    }

    /// Canonical events of every group, zero scalars dropped as `push` does.
    fn expanded_group_events(&self) -> Result<Vec<CompressionRelationEvent<E>>, AkitaError> {
        let mut events = Vec::new();
        for group in &self.groups.groups {
            let table =
                self.groups.tables.get(group.table).ok_or_else(|| {
                    AkitaError::Internal("compression group table is missing".into())
                })?;
            let values = table.values.get(..group.len).ok_or_else(|| {
                AkitaError::Internal("compression group exceeds its table".into())
            })?;
            for (index, &value) in values.iter().enumerate() {
                let scalar = group.scalar * value;
                if scalar.is_zero() {
                    continue;
                }
                events.push(CompressionRelationEvent {
                    physical_start: group.physical_start + index * group.stride,
                    coefficient_count: group.coefficient_count,
                    alpha_exponent_start: group.alpha_exponent_start,
                    scalar,
                });
            }
        }
        Ok(events)
    }

    /// Canonical dense accumulation plus the expanded groups.
    pub(super) fn accumulate_dense(&self, weights: &mut [E]) -> Result<(), AkitaError> {
        if weights.len() != self.physical_field_len {
            return Err(AkitaError::InvalidSize {
                expected: self.physical_field_len,
                actual: weights.len(),
            });
        }
        let expanded = self.expanded_group_events()?;
        for event in self.events.iter().chain(&expanded) {
            let alpha = self
                .alpha_powers
                .get(
                    event.alpha_exponent_start
                        ..event.alpha_exponent_start + event.coefficient_count,
                )
                .ok_or_else(|| {
                    AkitaError::Internal(
                        "compression event alpha span exceeds the power table".into(),
                    )
                })?;
            let target = weights
                .get_mut(event.physical_start..event.physical_start + event.coefficient_count)
                .ok_or_else(|| {
                    AkitaError::Internal(
                        "compression event physical span exceeds the dense weight table".into(),
                    )
                })?;
            for (weight, &power) in target.iter_mut().zip(alpha) {
                *weight += event.scalar * power;
            }
        }
        Ok(())
    }

    /// Consume the relation table into sorted nonzero physical entries
    /// (groups expanded, then the canonical clustering, verbatim).
    pub fn into_sparse_entries(mut self) -> Result<Vec<(usize, E)>, AkitaError> {
        /// Coefficients per parallel task, rounded up to whole clusters.
        const TASK_COEFFICIENTS: usize = 1 << 14;
        let expanded = self.expanded_group_events()?;
        self.events.extend(expanded);
        self.groups = Groups::default();
        self.events
            .sort_unstable_by_key(|event| event.physical_start);
        let mut tasks = Vec::new();
        let (mut task_start, mut task_coefficients, mut cluster_end) = (0, 0, 0);
        for (index, event) in self.events.iter().enumerate() {
            if event.physical_start >= cluster_end && task_coefficients >= TASK_COEFFICIENTS {
                tasks.push(task_start..index);
                (task_start, task_coefficients) = (index, 0);
            }
            task_coefficients += event.coefficient_count;
            cluster_end = cluster_end.max(event.physical_start + event.coefficient_count);
        }
        tasks.push(task_start..self.events.len());
        let events = &self.events;
        let alpha_powers = &self.alpha_powers;
        let entries = cfg_into_iter!(tasks)
            .map(|task| {
                let mut entries = Vec::new();
                let mut dense = Vec::new();
                let mut rest = &events[task];
                while let Some(first) = rest.first() {
                    let cluster_start = first.physical_start;
                    let mut cluster_end = cluster_start + first.coefficient_count;
                    let cluster_len = 1 + rest[1..]
                        .iter()
                        .take_while(|event| {
                            let overlaps = event.physical_start < cluster_end;
                            if overlaps {
                                cluster_end =
                                    cluster_end.max(event.physical_start + event.coefficient_count);
                            }
                            overlaps
                        })
                        .count();
                    let (cluster, tail) = rest.split_at(cluster_len);
                    rest = tail;
                    dense.clear();
                    dense.resize(cluster_end - cluster_start, E::zero());
                    for event in cluster {
                        let powers = alpha_powers
                            .get(event.alpha_exponent_start..)
                            .and_then(|powers| powers.get(..event.coefficient_count))
                            .ok_or_else(|| {
                                AkitaError::Internal(
                                    "sparse compression event alpha span exceeds the power table"
                                        .into(),
                                )
                            })?;
                        let offset = event.physical_start - cluster_start;
                        for (weight, &power) in dense[offset..].iter_mut().zip(powers) {
                            *weight += event.scalar * power;
                        }
                    }
                    entries.extend(
                        dense
                            .iter()
                            .enumerate()
                            .filter(|(_, weight)| !weight.is_zero())
                            .map(|(offset, &weight)| (cluster_start + offset, weight)),
                    );
                }
                Ok(entries)
            })
            .collect::<Result<Vec<_>, AkitaError>>()?;
        Ok(entries.concat())
    }

    /// Evaluate the table's multilinear extension at one full witness point:
    /// the canonical event evaluation plus every group by factor.
    #[tracing::instrument(skip_all, name = "compression_relation_mle")]
    pub fn evaluate_at_point(&self, point: &[E]) -> Result<E, AkitaError> {
        let events = self.evaluate_events_at_point(point)?;
        Ok(events + self.evaluate_groups_at_point(point)?)
    }

    /// The canonical `evaluate_at_point` over `events`, verbatim.
    fn evaluate_events_at_point(&self, point: &[E]) -> Result<E, AkitaError> {
        if checked::pow2(point.len()) != Some(self.physical_field_len) {
            return Err(AkitaError::InvalidSize {
                expected: self.physical_field_len.trailing_zeros() as usize,
                actual: point.len(),
            });
        }
        let mut fallback_equality = None;
        let mut low_factor_cache = Vec::new();
        let mut high_equality_cache = Vec::<(usize, OffsetEqWindow<E>)>::new();
        let mut evaluation = E::zero();
        let mut terms = [[E::zero(); 4]; 32];
        for batch in self.events.chunks(terms.len()) {
            for (event, term) in batch.iter().zip(&mut terms) {
                if event.physical_start & (event.coefficient_count - 1) != 0 {
                    if fallback_equality.is_none() {
                        fallback_equality = Some(OffsetEqWindow::new(point)?);
                    }
                    let equality = fallback_equality.as_ref().ok_or_else(|| {
                        AkitaError::Internal(
                            "compression fallback equality window was not initialized".into(),
                        )
                    })?;
                    let alpha_end = event
                        .alpha_exponent_start
                        .checked_add(event.coefficient_count)
                        .ok_or_else(|| {
                            AkitaError::Internal(
                                "unaligned compression event alpha extent overflow".into(),
                            )
                        })?;
                    let powers = self
                        .alpha_powers
                        .get(event.alpha_exponent_start..alpha_end)
                        .ok_or_else(|| {
                            AkitaError::Internal(
                                "unaligned compression event exceeds the alpha power table".into(),
                            )
                        })?;
                    let interval = powers.iter().copied().enumerate().fold(
                        E::zero(),
                        |sum, (offset, power)| {
                            sum + power * equality.eval(event.physical_start + offset)
                        },
                    );
                    *term = [event.scalar, E::one(), interval, E::one()];
                    continue;
                }
                let low_bits = event.coefficient_count.trailing_zeros() as usize;
                let cache_key = (event.alpha_exponent_start, event.coefficient_count);
                let low_factor = if let Some((_, value)) =
                    low_factor_cache.iter().find(|(key, _)| *key == cache_key)
                {
                    *value
                } else {
                    let alpha_end = event
                        .alpha_exponent_start
                        .checked_add(event.coefficient_count)
                        .ok_or_else(|| {
                            AkitaError::Internal(
                                "aligned compression event alpha extent overflow".into(),
                            )
                        })?;
                    let powers = self
                        .alpha_powers
                        .get(event.alpha_exponent_start..alpha_end)
                        .ok_or_else(|| {
                            AkitaError::Internal(
                                "aligned compression event exceeds the alpha power table".into(),
                            )
                        })?;
                    let low_point = point.get(..low_bits).ok_or_else(|| {
                        AkitaError::Internal(
                            "compression low point exceeds the checked point dimension".into(),
                        )
                    })?;
                    let value = multilinear_eval(powers, low_point)?;
                    low_factor_cache.push((cache_key, value));
                    value
                };
                let high_index = event.physical_start >> low_bits;
                let high_point = point.get(low_bits..).ok_or_else(|| {
                    AkitaError::Internal(
                        "compression high point exceeds the checked point dimension".into(),
                    )
                })?;
                let high_cache_index = if let Some(index) = high_equality_cache
                    .iter()
                    .position(|(cached_low_bits, _)| *cached_low_bits == low_bits)
                {
                    index
                } else {
                    let balanced_low_bits = high_point.len().div_ceil(2);
                    high_equality_cache.push((
                        low_bits,
                        OffsetEqWindow::with_low_bits(high_point, balanced_low_bits)?,
                    ));
                    high_equality_cache.len() - 1
                };
                let high_equality = high_equality_cache.get(high_cache_index).ok_or_else(|| {
                    AkitaError::Internal("compression high equality cache entry is missing".into())
                })?;
                let (eq_low, eq_high) = high_equality.1.eval_factors(high_index);
                *term = [event.scalar, low_factor, eq_low, eq_high];
            }
            evaluation += E::sum_of_products(&terms[..batch.len()]);
        }
        Ok(evaluation)
    }

    /// Sum of every group's events' extension at `point`, by factor.
    fn evaluate_groups_at_point(&self, point: &[E]) -> Result<E, AkitaError> {
        let mut low_factors = Vec::<((usize, usize), E)>::new();
        let mut windows = Vec::<(usize, OffsetEqWindow<E>)>::new();
        let mut blocks = Vec::<((usize, usize, usize, usize), E)>::new();
        let mut evaluation = E::zero();
        for group in &self.groups.groups {
            let count = group.coefficient_count;
            let low_bits = count.trailing_zeros() as usize;
            let key = (group.alpha_exponent_start, count);
            let low_factor =
                if let Some((_, value)) = low_factors.iter().find(|(cached, _)| *cached == key) {
                    *value
                } else {
                    let powers = group
                        .alpha_exponent_start
                        .checked_add(count)
                        .and_then(|end| self.alpha_powers.get(group.alpha_exponent_start..end))
                        .ok_or_else(|| {
                            AkitaError::Internal(
                                "compression group exceeds the alpha power table".into(),
                            )
                        })?;
                    let low_point = point.get(..low_bits).ok_or_else(|| {
                        AkitaError::Internal("compression group low point exceeds the point".into())
                    })?;
                    let value = multilinear_eval(powers, low_point)?;
                    low_factors.push((key, value));
                    value
                };
            let high_point = point.get(low_bits..).ok_or_else(|| {
                AkitaError::Internal("compression group high point exceeds the point".into())
            })?;
            let table =
                self.groups.tables.get(group.table).ok_or_else(|| {
                    AkitaError::Internal("compression group table is missing".into())
                })?;
            let values = table.values.get(..group.len).ok_or_else(|| {
                AkitaError::Internal("compression group exceeds its table".into())
            })?;
            let first_high = group.physical_start >> low_bits;
            let high_stride = group.stride >> low_bits;
            let sum = if group.len == 1 || high_stride.is_power_of_two() {
                let stride_bits = if group.len == 1 {
                    0
                } else {
                    high_stride.trailing_zeros() as usize
                };
                Self::dyadic_sum(
                    table,
                    values,
                    group.table,
                    high_point,
                    low_bits,
                    first_high,
                    stride_bits,
                    &mut blocks,
                )?
            } else {
                let window_index = if let Some(index) =
                    windows.iter().position(|(cached, _)| *cached == low_bits)
                {
                    index
                } else {
                    let balanced_low_bits = high_point.len().div_ceil(2);
                    windows.push((
                        low_bits,
                        OffsetEqWindow::with_low_bits(high_point, balanced_low_bits)?,
                    ));
                    windows.len() - 1
                };
                let window = &windows
                    .get(window_index)
                    .ok_or_else(|| {
                        AkitaError::Internal("compression group equality window is missing".into())
                    })?
                    .1;
                let mut sum = E::zero();
                let mut index = first_high;
                for &value in values {
                    let (eq_low, eq_high) = window.eval_factors(index);
                    sum += value * eq_low * eq_high;
                    index = index.wrapping_add(high_stride);
                }
                sum
            };
            evaluation += group.scalar * low_factor * sum;
        }
        Ok(evaluation)
    }

    /// `sum_j values[j] * eq(high_point, first_high + j * 2^stride_bits)` by
    /// dyadic blocks (FACTORIZATION.md, Lemmas 4 and 5).
    #[allow(clippy::too_many_arguments)]
    fn dyadic_sum(
        table: &GroupTable<E>,
        values: &[E],
        table_index: usize,
        high_point: &[E],
        low_bits: usize,
        first_high: usize,
        stride_bits: usize,
        blocks: &mut Vec<((usize, usize, usize, usize), E)>,
    ) -> Result<E, AkitaError> {
        let (stride_point, run_point) = if stride_bits <= high_point.len() {
            high_point.split_at(stride_bits)
        } else {
            return Err(AkitaError::Internal(
                "compression group stride exceeds the point".into(),
            ));
        };
        let low_mask = (1usize << stride_bits) - 1;
        let stride_factor = eq_bits(stride_point, first_high & low_mask);
        let start = first_high >> stride_bits;
        let end = start
            .checked_add(values.len())
            .ok_or_else(|| AkitaError::Internal("compression group run overflow".into()))?;
        let mut sum = E::zero();
        let mut position = start;
        let mut ratio_power = E::one();
        while position < end {
            let remaining = end - position;
            let size_bits = (usize::BITS - 1 - remaining.leading_zeros()) as usize;
            let block_bits = if position == 0 {
                size_bits
            } else {
                size_bits.min(position.trailing_zeros() as usize)
            };
            let block_len = 1usize << block_bits;
            let offset = position - start;
            let (block_point, rest_point) = if block_bits <= run_point.len() {
                run_point.split_at(block_bits)
            } else {
                return Err(AkitaError::Internal(
                    "compression group block exceeds the point".into(),
                ));
            };
            let high_factor = eq_bits(rest_point, position >> block_bits);
            let block = if let Some(squares) = &table.ratio_squares {
                let product = block_point.iter().zip(squares).fold(
                    E::one(),
                    |acc, (&coordinate, &square)| {
                        acc * ((E::one() - coordinate) + coordinate * square)
                    },
                );
                ratio_power * product
            } else {
                let key = (table_index, offset, block_bits, low_bits + stride_bits);
                if let Some((_, value)) = blocks.iter().find(|(cached, _)| *cached == key) {
                    *value
                } else {
                    let slice = values.get(offset..offset + block_len).ok_or_else(|| {
                        AkitaError::Internal("compression group block exceeds its table".into())
                    })?;
                    let value = multilinear_eval(slice, block_point)?;
                    blocks.push((key, value));
                    value
                }
            };
            sum += high_factor * block;
            if let Some(squares) = &table.ratio_squares {
                // Advance beta^(position - start) by beta^(2^block_bits) when
                // another block follows.
                if position + block_len < end {
                    let square = squares.get(block_bits).ok_or_else(|| {
                        AkitaError::Internal("compression group ratio square is missing".into())
                    })?;
                    ratio_power *= *square;
                }
            }
            position += block_len;
        }
        Ok(stride_factor * sum)
    }
}

/// Recomposition events grouped by (source row, coefficient block) over the
/// gadget bits, or the canonical loop when any group fails a precondition.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_recomposition<F, E>(
    weights: &mut CompressionRelationWeights<E>,
    span_start: usize,
    digit_ring_dim: usize,
    source_coefficients: usize,
    source_ring_dim: usize,
    source_row_start: usize,
    source_row_count: usize,
    field_bits: usize,
    row_weights: &[E],
) -> Result<(), AkitaError>
where
    F: Field + CanonicalEncoding,
    E: FpExtEncoding<F> + Ring + ExtField<F>,
{
    let planned = plan_recomposition_groups::<F, E>(
        weights,
        span_start,
        digit_ring_dim,
        source_coefficients,
        source_ring_dim,
        source_row_start,
        source_row_count,
        field_bits,
        row_weights,
    );
    match planned {
        Some(groups) => {
            weights.groups.groups.extend(groups);
            Ok(())
        }
        None => push_recomposition_canonical::<F, E>(
            weights,
            span_start,
            digit_ring_dim,
            source_coefficients,
            source_ring_dim,
            source_row_start,
            source_row_count,
            field_bits,
            row_weights,
        ),
    }
}

/// Groups for one canonical `push_recomposition` call, or `None` when the
/// canonical loop must run (any error, overflow, misalignment, or a table
/// with a zero entry).
#[allow(clippy::too_many_arguments)]
fn plan_recomposition_groups<F, E>(
    weights: &mut CompressionRelationWeights<E>,
    span_start: usize,
    digit_ring_dim: usize,
    source_coefficients: usize,
    source_ring_dim: usize,
    source_row_start: usize,
    source_row_count: usize,
    field_bits: usize,
    row_weights: &[E],
) -> Option<Vec<CompressionEventGroup<E>>>
where
    F: Field + CanonicalEncoding,
    E: FpExtEncoding<F> + Ring + ExtField<F>,
{
    if digit_ring_dim == 0
        || !source_ring_dim.is_multiple_of(digit_ring_dim)
        || source_row_count.checked_mul(source_ring_dim) != Some(source_coefficients)
    {
        return None;
    }
    let mut groups = Vec::new();
    if field_bits == 0 || source_row_count == 0 || source_ring_dim == 0 {
        return Some(groups);
    }
    // The canonical loop computes every start with checked additions after an
    // unchecked `bit * source_coefficients`; the largest one must not overflow.
    let largest = (field_bits - 1)
        .checked_mul(source_coefficients)
        .and_then(|offset| span_start.checked_add(offset))
        .and_then(|start| start.checked_add((source_row_count - 1).checked_mul(source_ring_dim)?))
        .and_then(|start| start.checked_add(source_ring_dim - digit_ring_dim));
    largest?;
    let table = weights.gadget_table::<F>(field_bits);
    if !weights.groups.tables.get(table)?.all_nonzero {
        return None;
    }
    for source_row in 0..source_row_count {
        let row_weight = *row_weights.get(source_row_start.checked_add(source_row)?)?;
        if row_weight.is_zero() {
            continue;
        }
        for coefficient_start in (0..source_ring_dim).step_by(digit_ring_dim) {
            let physical_start = span_start
                .checked_add(source_row * source_ring_dim)?
                .checked_add(coefficient_start)?;
            if !weights.admits_progression(
                physical_start,
                source_coefficients,
                field_bits,
                digit_ring_dim,
                coefficient_start,
            ) {
                return None;
            }
            groups.push(CompressionEventGroup {
                physical_start,
                stride: source_coefficients,
                len: field_bits,
                coefficient_count: digit_ring_dim,
                alpha_exponent_start: coefficient_start,
                scalar: -row_weight,
                table,
            });
        }
    }
    Some(groups)
}

/// The canonical `push_recomposition`, verbatim.
#[allow(clippy::too_many_arguments)]
fn push_recomposition_canonical<F, E>(
    weights: &mut CompressionRelationWeights<E>,
    span_start: usize,
    digit_ring_dim: usize,
    source_coefficients: usize,
    source_ring_dim: usize,
    source_row_start: usize,
    source_row_count: usize,
    field_bits: usize,
    row_weights: &[E],
) -> Result<(), AkitaError>
where
    F: Field + CanonicalEncoding,
    E: FpExtEncoding<F> + Ring + ExtField<F>,
{
    if !source_ring_dim.is_multiple_of(digit_ring_dim)
        || source_row_count.checked_mul(source_ring_dim) != Some(source_coefficients)
    {
        return Err(AkitaError::InvalidSetup(
            "compression recomposition geometry is malformed".into(),
        ));
    }
    for (bit, power) in gadget_row_scalars::<F>(field_bits, 1)
        .into_iter()
        .enumerate()
    {
        for source_row in 0..source_row_count {
            let row_weight = *row_weights
                .get(source_row_start + source_row)
                .ok_or_else(|| {
                    AkitaError::Internal(
                        "compression source row exceeds the generated row weight table".into(),
                    )
                })?;
            for coefficient_start in (0..source_ring_dim).step_by(digit_ring_dim) {
                let physical_start = span_start
                    .checked_add(bit * source_coefficients)
                    .and_then(|start| start.checked_add(source_row * source_ring_dim))
                    .and_then(|start| start.checked_add(coefficient_start))
                    .ok_or_else(|| {
                        AkitaError::InvalidSetup("compression witness offset overflow".into())
                    })?;
                weights.push(
                    physical_start,
                    digit_ring_dim,
                    coefficient_start,
                    -(row_weight * E::lift_base(power)),
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jolt_field::Prime128OffsetA7F7 as F;
    use jolt_field::{One, Zero};

    /// Deterministic full-width field elements (no external RNG).
    struct Stream(u64);

    impl Stream {
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn field(&mut self) -> F {
            let high = F::from_u64(self.next_u64());
            high * F::from_u64(self.next_u64()) + F::from_u64(self.next_u64())
        }

        fn point(&mut self, len: usize) -> Vec<F> {
            (0..len).map(|_| self.field()).collect()
        }
    }

    fn empty(
        physical_field_len: usize,
        alpha_len: usize,
        block: usize,
        stream: &mut Stream,
    ) -> CompressionRelationWeights<F> {
        CompressionRelationWeights {
            events: Vec::new(),
            alpha_powers: (0..alpha_len).map(|_| stream.field()).collect(),
            coefficient_block_len: block,
            physical_field_len,
            groups: Groups::default(),
        }
    }

    /// The same table built through the canonical loops only.
    fn canonical_twin(weights: &CompressionRelationWeights<F>) -> CompressionRelationWeights<F> {
        let mut twin = weights.clone();
        twin.events.extend(weights.expanded_group_events().unwrap());
        twin.groups = Groups::default();
        twin
    }

    /// Points with random, zero and one coordinates.
    fn points(n: usize, stream: &mut Stream) -> Vec<Vec<F>> {
        let mut out = vec![
            stream.point(n),
            stream.point(n),
            vec![F::zero(); n],
            vec![F::one(); n],
        ];
        let mut mixed = stream.point(n);
        for (index, coordinate) in mixed.iter_mut().enumerate() {
            if index % 3 == 0 {
                *coordinate = F::zero();
            } else if index % 3 == 1 {
                *coordinate = F::one();
            }
        }
        out.push(mixed);
        out
    }

    fn assert_all_agree(weights: &CompressionRelationWeights<F>, stream: &mut Stream, label: &str) {
        let n = weights.physical_field_len.trailing_zeros() as usize;
        let dense = weights.materialize_dense().unwrap();
        let twin = canonical_twin(weights);
        assert_eq!(twin.materialize_dense().unwrap(), dense, "{label}: dense");
        for point in points(n, stream) {
            let expected = multilinear_eval(&dense, &point).unwrap();
            assert_eq!(
                weights.evaluate_at_point(&point).unwrap(),
                expected,
                "{label}: factorized"
            );
            assert_eq!(
                twin.evaluate_events_at_point(&point).unwrap(),
                expected,
                "{label}: canonical"
            );
        }
        assert_eq!(
            weights.clone().into_sparse_entries().unwrap(),
            twin.into_sparse_entries().unwrap(),
            "{label}: sparse entries"
        );
    }

    /// Recomposition calls of many geometries agree with the canonical loop
    /// and the dense extension at random, zero, one and mixed points.
    #[test]
    fn l3_recomposition_groups_match_canonical() {
        let mut stream = Stream(0x9e37_79b9_7f4a_7c15);
        // (span_start, digit, source_coefficients, source_ring_dim, rows, field_bits, n)
        let cases = [
            (0usize, 4usize, 16usize, 16usize, 1usize, 8usize, 9usize), // dyadic, aligned
            (32, 4, 16, 16, 1, 8, 9),                                   // unaligned run start
            (0, 4, 48, 16, 3, 5, 10),                                   // stride not a power of two
            (64, 2, 8, 8, 1, 7, 9),                                     // clipped run (7 bits)
            (0, 1, 2, 2, 1, 128, 9),                                    // count 1 (k = 0), 128 bits
            (16, 8, 16, 8, 2, 3, 8),                                    // two rows
            (0, 4, 16, 16, 1, 1, 8),                                    // a single bit
            (0, 4, 16, 16, 1, 0, 8),                                    // no bits at all
        ];
        for (index, &(span, digit, source, ring, rows, bits, n)) in cases.iter().enumerate() {
            let mut weights = empty(1 << n, 64, 1, &mut stream);
            let mut canonical = weights.clone();
            let row_weights = stream.point(rows + 2);
            push_recomposition::<F, F>(
                &mut weights,
                span,
                digit,
                source,
                ring,
                1,
                rows,
                bits,
                &row_weights,
            )
            .unwrap();
            push_recomposition_canonical::<F, F>(
                &mut canonical,
                span,
                digit,
                source,
                ring,
                1,
                rows,
                bits,
                &row_weights,
            )
            .unwrap();
            assert!(
                weights.events.is_empty(),
                "case {index}: every recomposition is grouped"
            );
            // A raw canonical event in the same table.
            let scalar = stream.field();
            weights.push(0, 2, 2, scalar).unwrap();
            canonical.push(0, 2, 2, scalar).unwrap();
            assert_eq!(
                weights.materialize_dense().unwrap(),
                canonical.materialize_dense().unwrap(),
                "case {index}: grouped and canonical tables"
            );
            assert_all_agree(
                &weights,
                &mut stream,
                &format!("recomposition case {index}"),
            );
        }
    }

    /// Zero row weights record nothing, as the canonical loop.
    #[test]
    fn l3_zero_row_weight_records_nothing() {
        let mut stream = Stream(7);
        let mut weights = empty(1 << 9, 64, 1, &mut stream);
        push_recomposition::<F, F>(&mut weights, 0, 4, 16, 16, 0, 1, 8, &[F::zero()]).unwrap();
        assert!(weights.groups.groups.is_empty() && weights.events.is_empty());
        weights
            .push_column_group(0, 4, 8, F::zero(), 0, &stream.point(8))
            .unwrap();
        assert!(weights.groups.groups.is_empty() && weights.events.is_empty());
    }

    /// Column groups: shared tables, interior and trailing zeros, widths that
    /// are not powers of two, unaligned spans, and the canonical fallback.
    #[test]
    fn l3_column_groups_match_canonical() {
        let mut stream = Stream(0x1234_5678_9abc_def1);
        let n = 10;
        let mut weights = empty(1 << n, 64, 2, &mut stream);
        let mut columns = stream.point(12);
        columns[3] = F::zero();
        columns[10] = F::zero();
        columns[11] = F::zero();
        let mut canonical = weights.clone();
        for span in [0usize, 64, 128, 256] {
            let row_weight = stream.field();
            weights
                .push_column_group(span, 4, 12, row_weight, 0, &columns)
                .unwrap();
            canonical
                .push_columns_canonical(span, 4, 12, row_weight, &columns)
                .unwrap();
        }
        let other = stream.point(5);
        let row_weight = stream.field();
        weights
            .push_column_group(512, 8, 5, row_weight, 1, &other)
            .unwrap();
        canonical
            .push_columns_canonical(512, 8, 5, row_weight, &other)
            .unwrap();
        // Span not aligned to the ring dimension: canonical events, aligned
        // to the block length so `push` admits them.
        let misaligned = stream.point(3);
        let row_weight = stream.field();
        weights
            .push_column_group(2, 4, 3, row_weight, 2, &misaligned)
            .unwrap();
        canonical
            .push_columns_canonical(2, 4, 3, row_weight, &misaligned)
            .unwrap();
        assert!(
            !weights.events.is_empty(),
            "the misaligned span takes the canonical path"
        );
        assert!(
            !weights.groups.groups.is_empty(),
            "aligned spans are grouped"
        );
        assert_eq!(
            weights.materialize_dense().unwrap(),
            canonical.materialize_dense().unwrap()
        );
        assert_all_agree(&weights, &mut stream, "columns");
    }

    /// Errors are the canonical errors: a progression leaving the domain or a
    /// misaligned block falls back to the canonical loop and fails the same way.
    #[test]
    fn l3_errors_are_canonical() {
        let mut stream = Stream(99);
        let row_weights = stream.point(4);
        for (span, block) in [(1usize << 9, 1usize), (2, 4)] {
            let mut grouped = empty(1 << 9, 64, block, &mut stream);
            let mut canonical = grouped.clone();
            let a =
                push_recomposition::<F, F>(&mut grouped, span, 4, 16, 16, 0, 1, 8, &row_weights);
            let b = push_recomposition_canonical::<F, F>(
                &mut canonical,
                span,
                4,
                16,
                16,
                0,
                1,
                8,
                &row_weights,
            );
            assert_eq!(format!("{a:?}"), format!("{b:?}"));
            assert!(a.is_err());
        }
        let mut grouped = empty(1 << 6, 64, 1, &mut stream);
        let mut canonical = grouped.clone();
        let columns = stream.point(32);
        let a = grouped.push_column_group(0, 4, 32, F::one(), 0, &columns);
        let b = canonical.push_columns_canonical(0, 4, 32, F::one(), &columns);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(a.is_err());
        let a = grouped.push_column_group(0, 4, 9, F::one(), 1, &columns[..8]);
        let b = canonical.push_columns_canonical(0, 4, 9, F::one(), &columns[..8]);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    /// Direct groups exercise tiers the producers do not reach: stride zero
    /// (repeated starts), a non-geometric table under the dyadic tier, and a
    /// geometric table with ratio one.
    #[test]
    fn l3_direct_groups_cover_every_tier() {
        let mut stream = Stream(0xdead_beef);
        let n = 9;
        let mut weights = empty(1 << n, 64, 1, &mut stream);
        weights
            .groups
            .tables
            .push(GroupTable::new(stream.point(16), true));
        weights
            .groups
            .tables
            .push(GroupTable::new(vec![F::one(); 16], true));
        assert!(weights.groups.tables[0].ratio_squares.is_none());
        assert!(weights.groups.tables[1].ratio_squares.is_some());
        let scalar = stream.field();
        for (start, stride, len, count, table) in [
            (8usize, 0usize, 5usize, 4usize, 0usize),
            (16, 4, 16, 4, 0),
            (20, 8, 13, 4, 1),
            (0, 12, 9, 4, 1),
            (24, 8, 1, 8, 0),
        ] {
            weights.groups.groups.push(CompressionEventGroup {
                physical_start: start,
                stride,
                len,
                coefficient_count: count,
                alpha_exponent_start: 4,
                scalar,
                table,
            });
        }
        assert_all_agree(&weights, &mut stream, "direct groups");
    }

    /// The product of two Boolean tables' extensions is not the extension of
    /// their product (the pitfall the factorization avoids by splitting only
    /// disjoint digit positions).
    #[test]
    fn l3_same_variable_products_do_not_factor() {
        let table = [F::one(), F::zero()];
        let point = [F::from_u64(5)];
        let product =
            multilinear_eval(&table, &point).unwrap() * multilinear_eval(&table, &point).unwrap();
        let extension = multilinear_eval(&[F::one(), F::zero()], &point).unwrap();
        assert_ne!(product, extension);
        let groups_value = eq_bits(&point[..], 0);
        assert_eq!(groups_value, extension);
    }
}
