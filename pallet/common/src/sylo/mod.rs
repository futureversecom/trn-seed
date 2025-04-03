extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	traits::Get, BoundedVec, CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{fmt::Debug, prelude::*};

#[derive(
	CloneNoBound,
	RuntimeDebugNoBound,
	Encode,
	Decode,
	PartialEqNoBound,
	EqNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(StringLimit))]
pub struct ResolverId<StringLimit>
where
	StringLimit: Get<u32>,
{
	pub method: BoundedVec<u8, StringLimit>,
	pub identifier: BoundedVec<u8, StringLimit>,
}

impl<T: Get<u32>> ResolverId<T> {
	pub fn to_did(&self) -> Vec<u8> {
		let method = self.method.to_vec();
		let method = String::from_utf8_lossy(method.as_slice());

		let identifier = self.identifier.to_vec();
		let identifier = String::from_utf8_lossy(identifier.as_slice());

		format!("did:{method}:{identifier}").as_bytes().to_vec()
	}
}

pub type ServiceEndpoint<StringLimit> = BoundedVec<u8, StringLimit>;

#[derive(
	CloneNoBound, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxServiceEndpoints, StringLimit))]
pub struct Resolver<AccountId, MaxServiceEndpoints, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	MaxServiceEndpoints: Get<u32>,
	StringLimit: Get<u32>,
{
	pub controller: AccountId,
	pub service_endpoints: BoundedVec<ServiceEndpoint<StringLimit>, MaxServiceEndpoints>,
}

pub type DataId<StringLimit> = BoundedVec<u8, StringLimit>;

#[derive(
	Clone, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct ValidationEntry<BlockNumber>
where
	BlockNumber: Debug + PartialEq + Clone,
{
	pub checksum: H256,
	pub block: BlockNumber,
}

#[derive(
	CloneNoBound, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxResolvers, MaxTags, MaxEntries, StringLimit))]
pub struct ValidationRecord<AccountId, BlockNumber, MaxResolvers, MaxTags, MaxEntries, StringLimit>
where
	AccountId: Debug + PartialEq + Clone,
	BlockNumber: Debug + PartialEq + Clone,
	MaxResolvers: Get<u32>,
	MaxTags: Get<u32>,
	MaxEntries: Get<u32>,
	StringLimit: Get<u32>,
{
	pub author: AccountId,
	pub resolvers: BoundedVec<ResolverId<StringLimit>, MaxResolvers>,
	pub data_type: BoundedVec<u8, StringLimit>,
	pub tags: BoundedVec<BoundedVec<u8, StringLimit>, MaxTags>,
	pub entries: BoundedVec<ValidationEntry<BlockNumber>, MaxEntries>,
}

pub trait SyloDataVerificationProvider {
	type AccountId: Debug + PartialEq + Clone;
	type BlockNumber: Debug + PartialEq + Clone;
	type MaxResolvers: Get<u32>;
	type MaxTags: Get<u32>;
	type MaxEntries: Get<u32>;
	type StringLimit: Get<u32>;

	fn get_validation_record(
		author: &Self::AccountId,
		data_id: &BoundedVec<u8, Self::StringLimit>,
	) -> Option<
		ValidationRecord<
			Self::AccountId,
			Self::BlockNumber,
			Self::MaxResolvers,
			Self::MaxTags,
			Self::MaxEntries,
			Self::StringLimit,
		>,
	>;
}

#[derive(
	Clone, Copy, Encode, Decode, RuntimeDebugNoBound, PartialEqNoBound, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum DataPermission {
	VIEW,
	MODIFY,
	DISTRIBUTE,
}

pub trait SyloDataPermissionsProvider {
	type AccountId: Debug + PartialEq + Clone;
	type StringLimit: Get<u32>;

	fn has_permission(
		data_author: &Self::AccountId,
		data_id: &DataId<Self::StringLimit>,
		grantee: &Self::AccountId,
		permission: DataPermission,
	) -> bool;
}
