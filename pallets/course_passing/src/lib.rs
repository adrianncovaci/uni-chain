#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::{
		sp_runtime::traits::Hash,
		traits::{tokens::ExistenceRequirement, Currency, Randomness},
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Course<T: Config> {
		pub credits: u8,
		pub dna: [u8; 16],
		pub owner: AccountOf<T>,
		pub course_year: CourseYear,
		pub price: Option<BalanceOf<T>>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum CourseYear {
		First,
		Second,
		Third,
		Fourth,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
		type CourseRandomness: Randomness<Self::Hash, Self::BlockNumber>;
		#[pallet::constant]
		type MaxCoursesOwned: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		CountForCoursesOverflowed,
		CourseExists,
		ExceedMaxCourseOwned,
		ClaimerIsCourseOwner,
		TransferToSelf,
		CourseDoesNotExist,
		CourseUnclaimable,
		CannotClaim,
		NotCourseOwner,
		CourseBuyerIsNotOwner,
		CourseIsNotForSale,
		BidPriceTooLow,
		NotEnoughBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created(T::AccountId, T::Hash),
		Claimed(T::AccountId, T::AccountId, T::Hash),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
	}

	#[pallet::storage]
	#[pallet::getter(fn course_cnt)]
	pub(super) type CourseCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn courses)]
	pub(super) type Courses<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Course<T>>;

	#[pallet::storage]
	#[pallet::getter(fn courses_owned)]
	pub(super) type CoursesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Hash, T::MaxCoursesOwned>,
		ValueQuery,
	>;

	// Our pallet's genesis configuration.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub courses: Vec<(T::AccountId, [u8; 16], CourseYear, u8)>,
	}

	// Required to implement default for GenesisConfig.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { courses: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// When building a kitty from genesis config, we require the dna and gender to be supplied.
			for (acct, dna, course_year, credits) in &self.courses {
				let _ = <Pallet<T>>::mint(
					acct,
					Some(dna.clone()),
					Some(course_year.clone()),
					Some(*credits),
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn create_course(origin: OriginFor<T>) -> DispatchResult {
			let sender = frame_system::ensure_signed(origin)?;
			let course_id = Self::mint(&sender, None, None, None)?;
			log::info!("Course {:?} has been minted by {:?}", course_id, sender);
			Self::deposit_event(Event::Created(sender, course_id));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			course_id: T::Hash,
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_course_owner(&course_id, &sender)?, <Error<T>>::NotCourseOwner);
			let mut course = Self::courses(&course_id).ok_or(<Error<T>>::CourseDoesNotExist)?;
			course.price = new_price;
			<Courses<T>>::insert(&course_id, course);
			Self::deposit_event(Event::PriceSet(sender, course_id, new_price));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			course_id: T::Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_course_owner(&course_id, &sender)?, <Error<T>>::NotCourseOwner);
			ensure!(sender != to, <Error<T>>::TransferToSelf);

			let owned = <CoursesOwned<T>>::get(&sender);

			ensure!(
				(owned.len() as u32) < (T::MaxCoursesOwned::get()),
				<Error<T>>::ExceedMaxCourseOwned
			);

			Self::transfer_course_to(&course_id, &to)?;
			Self::deposit_event(Event::Claimed(sender, to, course_id));
			Ok(())
		}

		#[transactional]
		#[pallet::weight(100)]
		pub fn buy_course(
			origin: OriginFor<T>,
			course_id: T::Hash,
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let course = Self::courses(&course_id).ok_or(<Error<T>>::BidPriceTooLow)?;
			if let Some(sale_price) = course.price {
				ensure!(sale_price <= bid_price, <Error<T>>::CourseBuyerIsNotOwner);
			} else {
				Err(<Error<T>>::CourseIsNotForSale)?;
			}

			ensure!(T::Currency::free_balance(&sender) >= bid_price, <Error<T>>::NotEnoughBalance);
			let to_owned = <CoursesOwned<T>>::get(&sender);
			ensure!(
				(to_owned.len() as u32) < T::MaxCoursesOwned::get(),
				<Error<T>>::ExceedMaxCourseOwned
			);

			let seller = course.owner.clone();
			T::Currency::transfer(&sender, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			Self::transfer_course_to(&course_id, &sender)?;

			Self::deposit_event(Event::Bought(sender, seller, course_id, bid_price));

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn breed_course(
			origin: OriginFor<T>,
			first_parent: T::Hash,
			second_parent: T::Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Self::is_course_owner(&first_parent, &sender)?, <Error<T>>::CourseDoesNotExist);
			ensure!(
				Self::is_course_owner(&second_parent, &sender)?,
				<Error<T>>::CourseDoesNotExist
			);

			let new_dna = Self::breed_dna(&first_parent, &second_parent)?;

			Self::mint(&sender, Some(new_dna), None, None)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn gen_dna() -> [u8; 16] {
			let payload = (
				T::CourseRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::extrinsic_index().unwrap_or_default(),
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}

		pub fn breed_dna(parent1: &T::Hash, parent2: &T::Hash) -> Result<[u8; 16], Error<T>> {
			let dna1 = Self::courses(parent1).ok_or(<Error<T>>::CourseDoesNotExist)?.dna;
			let dna2 = Self::courses(parent2).ok_or(<Error<T>>::CourseDoesNotExist)?.dna;

			let mut new_dna = Self::gen_dna();
			for i in 0..new_dna.len() {
				new_dna[i] = (new_dna[i] & dna1[i]) | (!new_dna[i] & dna2[i]);
			}
			Ok(new_dna)
		}

		pub fn is_course_owner(course_id: &T::Hash, acct: &T::AccountId) -> Result<bool, Error<T>> {
			match Self::courses(course_id) {
				Some(course) => Ok(course.owner == *acct),
				None => Err(<Error<T>>::CourseDoesNotExist),
			}
		}

		#[transactional]
		pub fn transfer_course_to(course_id: &T::Hash, to: &T::AccountId) -> Result<(), Error<T>> {
			let mut course = Self::courses(&course_id).ok_or(<Error<T>>::CourseDoesNotExist)?;
			let prev_owner = course.owner.clone();
			<CoursesOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *course_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			})
			.map_err(|_| <Error<T>>::CourseDoesNotExist)?;
			course.owner = to.clone();
			course.price = None;

			<Courses<T>>::insert(course_id, course);
			<CoursesOwned<T>>::try_mutate(to, |vec| vec.try_push(*course_id))
				.map_err(|_| <Error<T>>::ExceedMaxCourseOwned)?;

			Ok(())
		}

		fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			course_year: Option<CourseYear>,
			credits: Option<u8>,
		) -> Result<T::Hash, Error<T>> {
			let course_year = match course_year {
				Some(x) => x,
				None => CourseYear::First,
			};
			let credits = match credits {
				Some(x) => x,
				None => 3,
			};
			let course = Course::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				course_year,
				owner: owner.clone(),
				credits,
				price: None,
			};

			let course_id = T::Hashing::hash_of(&course);

			let new_cnt =
				Self::course_cnt().checked_add(1).ok_or(<Error<T>>::CountForCoursesOverflowed)?;

			ensure!(Self::courses(&course_id) == None, <Error<T>>::CourseExists);

			<CoursesOwned<T>>::try_mutate(&owner, |course_vec| course_vec.try_push(course_id))
				.map_err(|_| <Error<T>>::ExceedMaxCourseOwned)?;

			<Courses<T>>::insert(course_id, course);
			<CourseCnt<T>>::put(new_cnt);

			Ok(course_id)
		}
	}
}
