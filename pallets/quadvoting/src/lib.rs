#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod tests;

/*
	Voting Pallet
	==============

	The voting pallet allows voting for groups of proposals (topics) in each era. Each era === 10 blocks.
	During an era, it's possible to vote for current active proposals which were proposed in the previous era.
	It's also possible to create new proposals which will become available by the next era.

	Available actions:

	- submit_topic:
			Here a signed user can submit a topic which is stored alongside it's hash.
			A fee is required, to submit a proposal.

	-- vote_topic:
			Allows you cote for a hash in the current era. A fee is charged for each vote,
			and is a function of the square of the number of votes you have for that topic multiplied by the default weight.

	- get_current_topics:
			Here a user can get all topics hashes which are available to be voted in the current era.

	- get_next_topics:
			Get the topics which are queued to be voted in the next era

	- get_topic_preimage:
			Get the details of a topic given it's hash

	- get_era_winners:
			Returns a map of the era number, to the hash that won in that era

*/

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::traits::{Hash, Zero},
		traits::{Currency, LockableCurrency, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{collections::btree_map::*, vec, vec::Vec, *};

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Topic<AccountId, Balance, BlockNumber> {
		data: Vec<u8>,
		provider: AccountId,
		deposit: Balance,
		since: BlockNumber,
	}

	#[pallet::config] // <-- Step 2. code block will replace this.
	/// Configure the pallet by specifying the parameters and types on which it depends.
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		// type Token: ReservableCurrency<Self::AccountId>;

		/// The number of blocks between each era.
		#[pallet::constant]
		type EraDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type OneBlock: Get<BlockNumberFor<Self>>;

		// The max allowed number of votes a single user can make
		#[pallet::constant]
		type MaxVotes: Get<u16>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewTopic { who: T::AccountId, topic_hash: T::Hash, deposit: BalanceOf<T> },
		NewEra { era: T::BlockNumber },
		NewVote { who: T::AccountId, topic_hash: T::Hash },
	}

	#[pallet::error] // <-- Step 4. code block will replace this.
	pub enum Error<T> {
		DuplicateTopic,
		InvalidTopicHash,
		VoterReachedMaxVotes,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::unbounded]
	#[pallet::getter(fn get_topic_preimage)]
	pub(super) type Topics<T: Config> = StorageMap<
		_,
		Identity,
		T::Hash,
		Topic<T::AccountId, BalanceOf<T>, T::BlockNumber>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::unbounded]
	#[pallet::getter(fn get_next_topics)]
	// TopicsNextEra holds the topics from the next era which will be available for voting in the
	// next era.
	pub(super) type TopicsNextEra<T: Config> = StorageValue<_, Vec<T::Hash>, OptionQuery>;

	#[pallet::storage]
	#[pallet::unbounded]
	#[pallet::getter(fn get_current_topics)]
	// TopicsCurrEra holds the topics from the current era which are already available to be voted
	// for.
	pub(super) type TopicsCurrEra<T: Config> = StorageValue<_, Vec<T::Hash>, OptionQuery>;

	#[pallet::storage]
	#[pallet::unbounded]
	#[pallet::getter(fn get_votes)]
	pub(super) type Votes<T: Config> =
		StorageMap<_, Blake2_128, T::BlockNumber, Vec<(T::Hash, T::AccountId)>, OptionQuery>;

	#[pallet::storage]
	#[pallet::unbounded]
	#[pallet::getter(fn get_winners)]
	pub(super) type Winners<T: Config> =
		StorageMap<_, Blake2_128, T::BlockNumber, T::Hash, OptionQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// on_initialize, we would perform the era book keeping. If it's the beginning of a new era,
		// then we calculate the winners of the last era's voting, store that to the storage, and
		// then then the next era storage is cleared, to prepare for accepting new proposals. and
		// the topics moved to the current era to be voted for.
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			let weight = 0x0;
			let era_duration = T::EraDuration::get();

			if (block_number % era_duration).is_zero() {
				Self::deposit_event(Event::<T>::NewEra { era: block_number });

				let prev_era = ((block_number - T::OneBlock::get()) / era_duration) * era_duration;
				let votes = <Votes<T>>::get(prev_era).unwrap_or(vec![]);

				let (topics, _): (Vec<T::Hash>, Vec<T::AccountId>) =
					votes.clone().into_iter().unzip();

				let mut counts = BTreeMap::new();
				for word in topics.iter() {
					*counts.entry(word).or_insert(0) += 1;
				}

				match counts.iter().max_by_key(|entry| entry.1) {
					None => (),
					Some((key, _)) => <Winners<T>>::set(prev_era, Some(**key)),
				};

				//  New era is starting.
				let nextera_hashes = <TopicsNextEra<T>>::get();

				// set the items in the next era into the current era, preparing for voting
				<TopicsCurrEra<T>>::set(nextera_hashes);

				// Set the topics in next era to empty
				<TopicsNextEra<T>>::set(None);
			};

			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_0)]
		pub fn submit_topic(origin: OriginFor<T>, topic_bytes: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let topic_hash = T::Hashing::hash(&topic_bytes[..]);
			ensure!(!<Topics<T>>::contains_key(&topic_hash), Error::<T>::DuplicateTopic);

			// FIXME: Either make the amaount a constant or a function of the size of their topic
			let deposit = <BalanceOf<T>>::from(10 as u32);
			T::Currency::reserve(&who, deposit)
				.map_err(|_| "locker can't afford to lock the amount requested")?;

			let now = <frame_system::Pallet<T>>::block_number();
			let topic = Topic { data: topic_bytes, provider: who.clone(), deposit, since: now };

			// Insert the topic into the general list of topics.
			<Topics<T>>::insert(topic_hash, topic);

			// Check if topic hash already exists
			let hashes = <TopicsNextEra<T>>::get().unwrap_or(vec![]);
			ensure!(!hashes.contains(&topic_hash), Error::<T>::DuplicateTopic);

			// Add topic to the next era.
			<TopicsNextEra<T>>::append(topic_hash);

			Self::deposit_event(Event::<T>::NewTopic { who, topic_hash, deposit });
			Ok(())
		}

		#[pallet::weight(1_0 + T::DbWeight::get().writes(1))]
		pub fn vote_topic(origin: OriginFor<T>, topic_hash: T::Hash) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let era_duration = T::EraDuration::get();
			let block_number = <frame_system::Pallet<T>>::block_number();
			let curr_era = ((block_number % era_duration) + era_duration) % era_duration;

			let votes = <Votes<T>>::get(curr_era).unwrap_or(vec![]);
			let (votes_by_topic_who, votes_by_who) =
				votes.iter().fold((0, 0), |(by_topic_user, by_user), (topic_local, who_local)| {
					if topic_local == &topic_hash && who_local == &who {
						(by_topic_user + 1, by_user + 1)
					} else if who_local == &who {
						(by_topic_user, by_user + 1)
					} else {
						(by_topic_user, by_user)
					}
				});
			ensure!(votes_by_who <= T::MaxVotes::get(), Error::<T>::VoterReachedMaxVotes);

			// NOTE: this is the number of votes plus 1 squared, to represent quadratic voting
			let fee = 10;
			let quadratic_voting_fee = ((votes_by_topic_who + 1) ^ 2) * fee;
			let deposit = <BalanceOf<T>>::from(quadratic_voting_fee as u32);
			T::Currency::reserve(&who, deposit)?;

			// Actually register a vote for the topic
			<Votes<T>>::append(block_number, (topic_hash, &who));

			Self::deposit_event(Event::<T>::NewVote { who, topic_hash });

			Ok(().into())
		}
	}
}
