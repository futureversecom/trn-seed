use crate::*;
use frame_support::{assert_ok, traits::OriginTrait, BoundedVec};
use frame_system::pallet_prelude::OriginFor;
use seed_primitives::MetadataScheme;

pub struct SftBuilder<T>
where
	T: Config + frame_system::Config,
{
	// Collection details
	pub owner: T::AccountId,
	pub name: String,
	pub metadata_scheme: MetadataScheme,
	pub royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,

	// Token details
	pub token_name: String,
	pub initial_issuance: Balance,
	pub max_issuance: Option<Balance>,
	pub token_owner: Option<T::AccountId>,
}

impl<T> SftBuilder<T>
where
	T: Config + frame_system::Config,
	<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId: From<T::AccountId>,
{
	pub fn new(owner: T::AccountId) -> Self {
		Self {
			owner,
			name: String::from("SFT Collection"),
			metadata_scheme: MetadataScheme::try_from(b"https://default.com/".as_slice()).unwrap(),
			royalties_schedule: None,
			token_name: String::from("SFT Token"),
			initial_issuance: 0,
			max_issuance: None,
			token_owner: None,
		}
	}

	pub fn name(mut self, name: &str) -> Self {
		self.name = String::from(name);
		self
	}

	pub fn metadata_scheme(mut self, metadata_scheme: MetadataScheme) -> Self {
		self.metadata_scheme = metadata_scheme;
		self
	}

	pub fn royalties_schedule(
		mut self,
		royalties_schedule: RoyaltiesSchedule<T::AccountId>,
	) -> Self {
		self.royalties_schedule = Some(royalties_schedule);
		self
	}

	pub fn token_name(mut self, token_name: &str) -> Self {
		self.token_name = String::from(token_name);
		self
	}

	pub fn initial_issuance(mut self, initial_issuance: Balance) -> Self {
		self.initial_issuance = initial_issuance;
		self
	}

	pub fn max_issuance(mut self, max_issuance: Balance) -> Self {
		self.max_issuance = Some(max_issuance);
		self
	}

	pub fn token_owner(mut self, token_owner: T::AccountId) -> Self {
		self.token_owner = Some(token_owner);
		self
	}

	pub fn build(self) -> TokenId {
		let collection_name = BoundedVec::truncate_from(self.name.as_bytes().to_vec());
		let token_name = BoundedVec::truncate_from(self.token_name.as_bytes().to_vec());

		let collection_id = Pallet::<T>::do_create_collection(
			self.owner.clone(),
			collection_name,
			self.metadata_scheme,
			self.royalties_schedule,
			OriginChain::Root,
		)
		.expect("Failed to create SFT collection");

		assert_ok!(Pallet::<T>::create_token(
			OriginFor::<T>::signed(self.owner),
			collection_id,
			token_name,
			self.initial_issuance,
			self.max_issuance,
			self.token_owner,
		));

		(collection_id, 0)
	}
}

// Returns the SftTokenBalance of an account which includes free and reserved balance
pub fn sft_balance_of<T: Config>(token_id: TokenId, who: &T::AccountId) -> SftTokenBalance {
	let token_info = TokenInfo::<T>::get(token_id).unwrap();
	token_info
		.owned_tokens
		.into_iter()
		.find(|(account, _)| account == who)
		.map(|(_, token_balance)| token_balance)
		.unwrap_or_default()
}
