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

	// Struct for holding Course information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Course<T: Config> {
		pub dna: [u8; 16], // Using 16 bytes to represent a course DNA
		pub price: Option<BalanceOf<T>>,
		pub course_year: CourseYear,
		pub owner: AccountOf<T>,
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

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the Courses pallet.
		type Currency: Currency<Self::AccountId>;

		/// The maximum amount of Courses a single account can own.
		#[pallet::constant]
		type MaxCoursesOwned: Get<u32>;

		/// The type of Randomness we want to specify for this pallet.
		type CourseRandomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		/// Handles arithmetic overflow when incrementing the Course counter.
		CountForCoursesOverflow,
		/// An account cannot own more Courses than `MaxCourseCount`.
		ExceedMaxCourseOwned,
		/// Buyer cannot be the owner.
		BuyerIsCourseOwner,
		/// Cannot transfer a course to its owner.
		TransferToSelf,
		/// This course already exists
		CourseExists,
		/// Handles checking whether the Course exists.
		CourseNotExist,
		/// Handles checking that the Course is owned by the account transferring, buying or setting a price for it.
		NotCourseOwner,
		/// Ensures the Course is for sale.
		CourseNotForSale,
		/// Ensures that the buying price is greater than the asking price.
		CourseBidPriceTooLow,
		/// Ensures that an account has enough funds to purchase a Course.
		NotEnoughBalance,
	}

	// Events.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new Course was successfully created. \[sender, course_id\]
		Created(T::AccountId, T::Hash),
		/// Course price was successfully set. \[sender, course_id, new_price\]
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		/// A Course was successfully transferred. \[from, to, course_id\]
		Transferred(T::AccountId, T::AccountId, T::Hash),
		/// A Course was successfully bought. \[buyer, seller, course_id, bid_price\]
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
	}

	// Storage items.

	#[pallet::storage]
	#[pallet::getter(fn count_for_courses)]
	/// Keeps track of the number of Courses in existence.
	pub(super) type CountForCourses<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn courses)]
	/// Stores a Course's unique traits, owner and price.
	pub(super) type Courses<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Course<T>>;

	#[pallet::storage]
	#[pallet::getter(fn courses_owned)]
	/// Keeps track of what accounts own what Course.
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
		pub courses: Vec<(T::AccountId, [u8; 16], CourseYear)>,
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
			for (acct, dna, course_year) in &self.courses {
				let _ = <Pallet<T>>::mint(acct, Some(dna.clone()), Some(course_year.clone()));
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new unique course.
		///
		/// The actual course creation is done in the `mint()` function.
		#[pallet::weight(100)]
		pub fn create_course(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let course_id = Self::mint(&sender, None, None)?;

			// Logging to the console
			log::info!("A course is born with ID: {:?}.", course_id);
			// Deposit our "Created" event.
			Self::deposit_event(Event::Created(sender, course_id));
			Ok(())
		}

		/// Set the price for a Course.
		///
		/// Updates Course price and updates storage.
		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			course_id: T::Hash,
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Ensure the course exists and is called by the course owner
			ensure!(Self::is_course_owner(&course_id, &sender)?, <Error<T>>::NotCourseOwner);

			let mut course = Self::courses(&course_id).ok_or(<Error<T>>::CourseNotExist)?;

			course.price = new_price.clone();
			<Courses<T>>::insert(&course_id, course);

			// Deposit a "PriceSet" event.
			Self::deposit_event(Event::PriceSet(sender, course_id, new_price));

			Ok(())
		}

		/// Directly transfer a course to another recipient.
		///
		/// Any account that holds a course can send it to another Account. This will reset the asking
		/// price of the course, marking it not for sale.
		#[pallet::weight(100)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			course_id: T::Hash,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			// Ensure the course exists and is called by the course owner
			ensure!(Self::is_course_owner(&course_id, &from)?, <Error<T>>::NotCourseOwner);

			// Verify the course is not transferring back to its owner.
			ensure!(from != to, <Error<T>>::TransferToSelf);

			// Verify the recipient has the capacity to receive one more course
			let to_owned = <CoursesOwned<T>>::get(&to);
			ensure!(
				(to_owned.len() as u32) < T::MaxCoursesOwned::get(),
				<Error<T>>::ExceedMaxCourseOwned
			);

			Self::transfer_course_to(&course_id, &to)?;

			Self::deposit_event(Event::Transferred(from, to, course_id));

			Ok(())
		}

		/// Buy a saleable Course. The bid price provided from the buyer has to be equal or higher
		/// than the ask price from the seller.
		///
		/// This will reset the asking price of the course, marking it not for sale.
		/// Marking this method `transactional` so when an error is returned, we ensure no storage is changed.
		#[transactional]
		#[pallet::weight(100)]
		pub fn buy_course(
			origin: OriginFor<T>,
			course_id: T::Hash,
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			// Check the course exists and buyer is not the current course owner
			let course = Self::courses(&course_id).ok_or(<Error<T>>::CourseNotExist)?;
			ensure!(course.owner != buyer, <Error<T>>::BuyerIsCourseOwner);

			// Check the course is for sale and the course ask price <= bid_price
			if let Some(ask_price) = course.price {
				ensure!(ask_price <= bid_price, <Error<T>>::CourseBidPriceTooLow);
			} else {
				Err(<Error<T>>::CourseNotForSale)?;
			}

			// Check the buyer has enough free balance
			ensure!(T::Currency::free_balance(&buyer) >= bid_price, <Error<T>>::NotEnoughBalance);

			// Verify the buyer has the capacity to receive one more course
			let to_owned = <CoursesOwned<T>>::get(&buyer);
			ensure!(
				(to_owned.len() as u32) < T::MaxCoursesOwned::get(),
				<Error<T>>::ExceedMaxCourseOwned
			);

			let seller = course.owner.clone();

			// Transfer the amount from buyer to seller
			T::Currency::transfer(&buyer, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			// Transfer the course from seller to buyer
			Self::transfer_course_to(&course_id, &buyer)?;

			Self::deposit_event(Event::Bought(buyer, seller, course_id, bid_price));

			Ok(())
		}

		/// Breed a Course.
		///
		/// Breed two courses to create a new generation
		/// of Courses.
		#[pallet::weight(100)]
		pub fn breed_course(
			origin: OriginFor<T>,
			parent1: T::Hash,
			parent2: T::Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Check: Verify `sender` owns both courses (and both courses exist).
			ensure!(Self::is_course_owner(&parent1, &sender)?, <Error<T>>::NotCourseOwner);
			ensure!(Self::is_course_owner(&parent2, &sender)?, <Error<T>>::NotCourseOwner);

			let new_dna = Self::breed_dna(&parent1, &parent2)?;
			Self::mint(&sender, Some(new_dna), None)?;

			Ok(())
		}
	}

	//** Our helper functions.**//

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
			let dna1 = Self::courses(parent1).ok_or(<Error<T>>::CourseNotExist)?.dna;
			let dna2 = Self::courses(parent2).ok_or(<Error<T>>::CourseNotExist)?.dna;

			let mut new_dna = Self::gen_dna();
			for i in 0..new_dna.len() {
				new_dna[i] = (new_dna[i] & dna1[i]) | (!new_dna[i] & dna2[i]);
			}
			Ok(new_dna)
		}

		// Helper to mint a Course.
		pub fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			course_year: Option<CourseYear>,
		) -> Result<T::Hash, Error<T>> {
			let course_year = match course_year {
				Some(x) => x,
				None => CourseYear::First,
			};

			let course = Course::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				course_year,
				owner: owner.clone(),
			};

			let course_id = T::Hashing::hash_of(&course);

			// Performs this operation first as it may fail
			let new_cnt = Self::count_for_courses()
				.checked_add(1)
				.ok_or(<Error<T>>::CountForCoursesOverflow)?;

			// Check if the course does not already exist in our storage map
			ensure!(Self::courses(&course_id) == None, <Error<T>>::CourseExists);

			// Performs this operation first because as it may fail
			<CoursesOwned<T>>::try_mutate(&owner, |course_vec| course_vec.try_push(course_id))
				.map_err(|_| <Error<T>>::ExceedMaxCourseOwned)?;

			<Courses<T>>::insert(course_id, course);
			<CountForCourses<T>>::put(new_cnt);
			Ok(course_id)
		}

		pub fn is_course_owner(course_id: &T::Hash, acct: &T::AccountId) -> Result<bool, Error<T>> {
			match Self::courses(course_id) {
				Some(course) => Ok(course.owner == *acct),
				None => Err(<Error<T>>::CourseNotExist),
			}
		}

		#[transactional]
		pub fn transfer_course_to(course_id: &T::Hash, to: &T::AccountId) -> Result<(), Error<T>> {
			let mut course = Self::courses(&course_id).ok_or(<Error<T>>::CourseNotExist)?;

			let prev_owner = course.owner.clone();

			// Remove `course_id` from the CourseOwned vector of `prev_course_owner`
			<CoursesOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *course_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			})
			.map_err(|_| <Error<T>>::CourseNotExist)?;

			// Update the course owner
			course.owner = to.clone();
			// Reset the ask price so the course is not for sale until `set_price()` is called
			// by the current owner.
			course.price = None;

			<Courses<T>>::insert(course_id, course);

			<CoursesOwned<T>>::try_mutate(to, |vec| vec.try_push(*course_id))
				.map_err(|_| <Error<T>>::ExceedMaxCourseOwned)?;

			Ok(())
		}
	}
}
