use crate::*;
use frame_support::{assert_ok, traits::OriginTrait, BoundedVec};
use frame_system::pallet_prelude::OriginFor;
use seed_primitives::{CrossChainCompatibility, MetadataScheme, TokenCount};

pub struct NftBuilder<T>
where
	T: Config + frame_system::Config,
{
	pub owner: T::AccountId,
	pub name: String,
	pub initial_issuance: TokenCount,
	pub max_issuance: Option<TokenCount>,
	pub token_owner: Option<T::AccountId>,
	pub metadata_scheme: MetadataScheme,
	pub royalties_schedule: Option<RoyaltiesSchedule<T::AccountId>>,
	pub cross_chain_compatibility: CrossChainCompatibility,
}

impl<T> NftBuilder<T>
where
	T: Config + frame_system::Config,
	<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId: From<T::AccountId>,
{
	pub fn new(owner: T::AccountId) -> Self {
		Self {
			owner,
			name: String::from("NFT Collection"),
			initial_issuance: 0,
			max_issuance: None,
			token_owner: None,
			metadata_scheme: MetadataScheme::try_from(b"https://example.com/".as_slice()).unwrap(),
			royalties_schedule: None,
			cross_chain_compatibility: Default::default(),
		}
	}

	pub fn name(mut self, name: &str) -> Self {
		self.name = String::from(name);
		self
	}

	pub fn initial_issuance(mut self, initial_issuance: TokenCount) -> Self {
		self.initial_issuance = initial_issuance;
		self
	}

	pub fn max_issuance(mut self, max_issuance: TokenCount) -> Self {
		self.max_issuance = Some(max_issuance);
		self
	}

	pub fn token_owner(mut self, token_owner: T::AccountId) -> Self {
		self.token_owner = Some(token_owner);
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

	pub fn cross_chain_compatibility(
		mut self,
		cross_chain_compatibility: CrossChainCompatibility,
	) -> Self {
		self.cross_chain_compatibility = cross_chain_compatibility;
		self
	}

	pub fn build(self) -> CollectionUuid {
		let collection_id = Pallet::<T>::next_collection_uuid().unwrap();
		let collection_name = BoundedVec::truncate_from(self.name.as_bytes().to_vec());
		assert_ok!(Pallet::<T>::create_collection(
			OriginFor::<T>::signed(self.owner),
			collection_name,
			self.initial_issuance,
			self.max_issuance,
			self.token_owner,
			self.metadata_scheme,
			self.royalties_schedule,
			self.cross_chain_compatibility,
		));
		collection_id
	}
}
