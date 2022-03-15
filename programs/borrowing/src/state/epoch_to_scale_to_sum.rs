use anchor_lang::{prelude::ProgramError, Loader};

use crate::{BorrowError, EpochToScaleToSumAccount, StabilityTokenMap};
use std::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct EpochToScaleToSum {
    pub hmap: Vec<Vec<StabilityTokenMap>>,
}

pub(crate) enum LoadingMode {
    Init,
    Mut,
}

macro_rules! unpack_epoch {
    ($data:ident) => {
        // TODO: scale this up a lot more.
        // this is essentially a 3D array
        // [[[sum]]]    -> first layer is the epoch layer
        //              -> second layer is the
        // [
        //   total_length, num_epochs,
        //     this_epoch_length, data,
        //     this_epoch_length, data,
        //     this_epoch_length, data,
        //     this_epoch_length, data,
        //     ...
        //  ]

        {
            let mut hmap: Vec<Vec<StabilityTokenMap>> = vec![];

            let num_epochs = $data[1];

            let mut current_cursor = 1;
            for _ in 0..num_epochs {
                let mut scale: Vec<StabilityTokenMap> = vec![];
                current_cursor += 1;
                let scale_length = $data[current_cursor];

                for _ in 0..scale_length {
                    scale.push(StabilityTokenMap {
                        sol: $data[current_cursor + 1],
                        eth: $data[current_cursor + 2],
                        btc: $data[current_cursor + 3],
                        srm: $data[current_cursor + 4],
                        ray: $data[current_cursor + 5],
                        ftt: $data[current_cursor + 6],
                        hbb: $data[current_cursor + 7],
                    });
                    current_cursor += 7;
                }

                hmap.push(scale);
            }
            hmap
        }
    };
}

macro_rules! pack_epoch {
    ($data:ident, $hmap:ident) => {
        let num_epochs = $hmap.len();
        let mut total_length = 2;

        let mut current_cursor = 1;
        let num_coins = 6;

        for epoch in 0..num_epochs {
            let scale_length = $hmap[epoch].len() as u128;
            total_length += 1 + (scale_length * num_coins);

            current_cursor += 1;
            $data[current_cursor] = scale_length;

            for sum in $hmap[epoch].iter() {
                $data[current_cursor + 1] = (*sum).sol;
                $data[current_cursor + 2] = (*sum).eth;
                $data[current_cursor + 3] = (*sum).btc;
                $data[current_cursor + 4] = (*sum).srm;
                $data[current_cursor + 5] = (*sum).ray;
                $data[current_cursor + 6] = (*sum).ftt;
                $data[current_cursor + 7] = (*sum).hbb;
                current_cursor += 7;
            }
        }

        $data[0] = total_length;
        $data[1] = num_epochs as u128;
    };
}

impl EpochToScaleToSum {
    pub fn default() -> Self {
        EpochToScaleToSum {
            hmap: vec![vec![StabilityTokenMap::default()]],
        }
    }

    pub fn get_sum(&self, epoch: u64, scale: u64) -> Option<StabilityTokenMap> {
        let e = usize::try_from(epoch).unwrap();
        let s = usize::try_from(scale).unwrap();
        if e < self.hmap.len() {
            if let Some(v) = self.hmap[e].get(s) {
                return Some(*v);
            }
        }
        None
    }

    pub fn set_sum(
        &mut self,
        epoch: u64,
        scale: u64,
        sum: StabilityTokenMap,
    ) -> Result<(), crate::BorrowError> {
        let epoch = usize::try_from(epoch).unwrap();
        let scale = usize::try_from(scale).unwrap();
        match epoch {
            e if e == self.hmap.len() - 1 => {
                // same current epoch
                match scale {
                    s if s == self.hmap[e].len() - 1 => {
                        self.hmap[e][s] = sum;
                    }
                    s if s == self.hmap[e].len() => {
                        self.hmap[e].push(sum);
                    }
                    _ => {
                        return Err(BorrowError::CannotGenerateSeed);
                    }
                }
            }
            e if e == self.hmap.len() => {
                // new epoch
                if scale != 0 {
                    return Err(BorrowError::CannotGenerateSeed);
                }
                self.hmap.push(vec![sum]);
            }
            _ => {
                // plain wrong, should never happen
                return Err(BorrowError::CannotGenerateSeed);
            }
        };

        Ok(())
    }

    // #[cfg(test)]
    #[allow(dead_code)]
    pub fn from(v: Vec<Vec<StabilityTokenMap>>) -> Self {
        EpochToScaleToSum { hmap: v }
    }
    #[allow(dead_code)]
    pub fn unpack(data: &[u128; 1000]) -> Self {
        let hmap = unpack_epoch!(data);

        Self { hmap }
    }

    pub fn unpack_from_zero_copy_account(
        epoch_to_scale_to_sum_account: &Loader<EpochToScaleToSumAccount>,
    ) -> Result<Self, ProgramError> {
        let account = &epoch_to_scale_to_sum_account.load()?;
        let data = &account.data;
        let hmap = unpack_epoch!(data);
        Ok(Self { hmap })
    }

    #[allow(dead_code)]
    pub fn pack(&self) -> [u128; 1000] {
        let mut data: [u128; 1000] = [0; 1000];
        let hmap = &self.hmap;
        pack_epoch!(data, hmap);
        data
    }

    pub(crate) fn pack_to_zero_copy_account(
        &self,
        epoch_to_scale_to_sum_account: &mut Loader<EpochToScaleToSumAccount>,
        mode: LoadingMode,
    ) -> Result<(), crate::BorrowError> {
        let account = &mut (match mode {
            LoadingMode::Init => epoch_to_scale_to_sum_account
                .load_init()
                .map_err(|_e| crate::BorrowError::CannotDeserializeSumMap)?,
            LoadingMode::Mut => epoch_to_scale_to_sum_account
                .load_mut()
                .map_err(|_e| crate::BorrowError::CannotDeserializeSumMap)?,
        });

        // let account = &mut epoch_to_scale_to_sum_account.load_mut()?;
        // let mut data: [u128; 1000] = [0; 1000];

        let num_epochs = self.hmap.len();
        let mut total_length = 2;

        let mut current_cursor = 1;
        let num_coins = 6;

        for epoch in 0..num_epochs {
            let scale_length = self.hmap[epoch].len() as u128;
            total_length += 1 + (scale_length * num_coins);

            current_cursor += 1;
            account.data[current_cursor] = scale_length;

            for sum in self.hmap[epoch].iter() {
                account.data[current_cursor + 1] = (*sum).sol;
                account.data[current_cursor + 2] = (*sum).eth;
                account.data[current_cursor + 3] = (*sum).btc;
                account.data[current_cursor + 4] = (*sum).srm;
                account.data[current_cursor + 5] = (*sum).ray;
                account.data[current_cursor + 6] = (*sum).ftt;
                account.data[current_cursor + 7] = (*sum).hbb;
                current_cursor += 7;
            }
        }

        account.data[0] = total_length;
        account.data[1] = num_epochs as u128;

        // let mut data = &mut account.data;
        // let hmap = &self.hmap;

        // pack_epoch!(data, hmap);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::StabilityTokenMap;

    use super::EpochToScaleToSum;

    #[test]
    fn test_epoch_serialize_default() {
        let default = EpochToScaleToSum::default();

        let serialized_once = default.pack();
        let deserialized_once = EpochToScaleToSum::unpack(&serialized_once);
        let serialized_twice = deserialized_once.pack();
        let deserialized_twice = EpochToScaleToSum::unpack(&serialized_twice);
        let serialized_thrice = deserialized_twice.pack();

        assert_eq!(default, deserialized_once);
        assert_eq!(default, deserialized_twice);

        assert_eq!(serialized_once, serialized_twice);
        assert_eq!(serialized_twice, serialized_thrice);
    }

    #[test]
    fn test_epoch_serialize_simple() {
        let expected = EpochToScaleToSum::from(vec![vec![StabilityTokenMap::new(
            100, 110, 120, 130, 7, 10, 5,
        )]]);

        let serialized_once = expected.pack();
        let deserialized_once = EpochToScaleToSum::unpack(&serialized_once);
        let serialized_twice = deserialized_once.pack();
        let deserialized_twice = EpochToScaleToSum::unpack(&serialized_twice);
        let serialized_thrice = deserialized_twice.pack();

        assert_eq!(expected, deserialized_once);
        assert_eq!(expected, deserialized_twice);

        assert_eq!(serialized_once, serialized_twice);
        assert_eq!(serialized_twice, serialized_thrice);

        // println!(
        //     "Default {:?} DeserializedTwice {:?}",
        //     expected, deserialized_twice
        // );
    }
    #[test]
    fn test_epoch_serialize_multi() {
        let expected = EpochToScaleToSum::from(vec![
            vec![
                StabilityTokenMap::new(0, 100, 110, 120, 130, 5, 1),
                StabilityTokenMap::new(2, 100, 110, 120, 130, 9, 12),
            ],
            vec![
                StabilityTokenMap::new(7, 100, 110, 120, 830, 2, 99),
                StabilityTokenMap::new(7, 100, 210, 120, 130, 2, 99),
                StabilityTokenMap::new(7, 100, 310, 120, 430, 2, 99),
                StabilityTokenMap::new(7, 100, 410, 120, 230, 89, 2),
                StabilityTokenMap::new(7, 100, 410, 120, 230, 89, 2),
                StabilityTokenMap::new(7, 100, 510, 120, 2230, 89, 2),
            ],
            vec![
                StabilityTokenMap::new(9, 100, 110, 120, 12330, 6, 7),
                StabilityTokenMap::new(9, 100, 110, 120, 52330, 6, 7),
                StabilityTokenMap::new(9, 100, 110, 120, 63230, 6, 7),
            ],
        ]);

        let serialized_once = expected.pack();
        let deserialized_once = EpochToScaleToSum::unpack(&serialized_once);
        let serialized_twice = deserialized_once.pack();
        let deserialized_twice = EpochToScaleToSum::unpack(&serialized_twice);
        let serialized_thrice = deserialized_twice.pack();

        assert_eq!(expected, deserialized_once);
        assert_eq!(expected, deserialized_twice);

        assert_eq!(serialized_once, serialized_twice);
        assert_eq!(serialized_twice, serialized_thrice);

        // println!(
        //     "Default {:?} DeserializedTwice {:?}",
        //     expected, deserialized_twice
        // );

        println!("Data {:?}", serialized_thrice);
    }

    #[test]
    fn test_epoch_serialize_dezerialize_and_accessor() {
        let data = [
            71, 3, 2, 0, 100, 110, 120, 130, 5, 1, 2, 100, 110, 120, 130, 9, 12, 6, 7, 100, 110,
            120, 830, 2, 99, 7, 100, 210, 120, 130, 2, 99, 7, 100, 310, 120, 430, 2, 99, 7, 100,
            410, 120, 230, 89, 2, 7, 100, 410, 120, 230, 89, 2, 7, 100, 510, 120, 2230, 89, 2, 3,
            9, 100, 110, 120, 12330, 6, 7, 9, 100, 110, 120, 52330, 6, 7, 9, 100, 110, 120, 63230,
            6, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual = EpochToScaleToSum::unpack(&data);

        let actual_token_map = actual.get_sum(1, 5);
        let expected_token_map = StabilityTokenMap::new(7, 100, 510, 120, 2230, 89, 2);
        assert_eq!(actual_token_map, Some(expected_token_map));
    }

    #[ignore]
    #[test]
    #[rustfmt::skip]
    fn test_epoch_serialize_dezerialize_and_accessor_two() {
        let data = [
            30, 4, 1, 0, 0, 1485074441000000000, 1485074441000000000, 2475123475000000000, 1485074441000000000,
            1, 0, 0, 1485074516000000000, 990049677000000000, 990049677000000000, 1485074516000000000, 1, 0, 0, 1484926196000000000, 2445339545000000000, 2445339545000000000, 1484926196000000000, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        ];

        let actual = EpochToScaleToSum::unpack(&data);
        println!("Actual {:?}", actual);

        let serialized_once = actual.pack();


        // let actual_token_map = actual.get_sum(1, 1);
        // let expected_token_map = StabilityTokenMap::new(100, 510, 120, 2230, 89, 2);
        assert_eq!(serialized_once, data);
    }
}
