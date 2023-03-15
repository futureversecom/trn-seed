#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::pallet_prelude::*;

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::storage]
	pub type MyValue<T> = StorageValue<_, NewType, ValueQuery>;

	#[pallet::storage]
	pub type MyMap<T: Config> = StorageMap<_, Twox64Concat, u32, NewType, ValueQuery>;

	#[pallet::event]
	pub enum Event<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo, MaxEncodedLen, Default)]
pub struct NewType {
	pub value: u32,
}

#[derive(Decode, Encode, Debug, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct NewTemplateType<AccountId> {
	address: AccountId,
}
