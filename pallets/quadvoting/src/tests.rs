use crate::mock::*;
use frame_support::{assert_ok, traits::Hooks};

/// Run until a particular block.
pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		// QuadVoting::on_initialize(System::block_number());
	}
}

#[test]
fn submit_topic_with_sufficient_funds() {
	new_test_ext().execute_with(|| {
		assert_ok!(QuadVoting::submit_topic(Origin::signed(1), vec![0]));
	});
}

#[test]
fn test_voting_end_to_end() {
	new_test_ext().execute_with(|| {
		// Starting at the initial block
		System::set_block_number(0);
		System::on_initialize(System::block_number());
		// Confirm that no votes or topics exist
		assert!(QuadVoting::get_votes(System::block_number()).is_none());
		assert!(QuadVoting::get_current_topics().is_none());
		assert!(QuadVoting::get_next_topics().is_none());

		// Let's create new topics. New topics should be created on the next topics batch until
		assert!(QuadVoting::submit_topic(
			Origin::signed(1),
			"new test topic 1".as_bytes().to_vec()
		)
		.is_ok());
		assert!(
			QuadVoting::get_current_topics().is_none(),
			"confirm that the newly added topic wasn't added to current topics"
		);
		assert!(
			QuadVoting::get_next_topics().is_some(),
			"confirm that the newly added topic was added to next topics batch"
		);

		// Now we've confirmed that the new topic is in the next batch. Let's create new topics
		assert!(QuadVoting::submit_topic(
			Origin::signed(1),
			"new test topic 2".as_bytes().to_vec()
		)
		.is_ok());
		assert!(QuadVoting::submit_topic(
			Origin::signed(1),
			"new test topic 3".as_bytes().to_vec()
		)
		.is_ok());

		// Next we transition to a new era.
		run_to_block(20);
		QuadVoting::on_initialize(System::block_number());

		// At start of new era, next topics should now be empty
		assert!(QuadVoting::get_next_topics().is_none()); //.expect("should have current topics");

		// the former next topics should now be in currnet topic
		let current_topics = QuadVoting::get_current_topics().expect("should have 3 topics");
		assert_eq!(current_topics.len(), 3);

		// Vote for item 1 and 3
		assert_ok!(QuadVoting::vote_topic(Origin::signed(1), current_topics[0]));
		assert_ok!(QuadVoting::vote_topic(Origin::signed(1), current_topics[2]));

		// Use a different user to vote only item 2
		assert_ok!(QuadVoting::vote_topic(Origin::signed(2), current_topics[1]));
		assert_ok!(QuadVoting::vote_topic(Origin::signed(2), current_topics[2]));

		// Get votes for blcok
		let votes = QuadVoting::get_votes(System::block_number()).expect("should have votes");
		assert_eq!(votes.len(), 4);

		run_to_block(40);
		QuadVoting::on_initialize(System::block_number());

		let winner = QuadVoting::get_winners(20).expect("should have some winners");
		// The winner for the block 20 era should be topic 2 as it had the most votes.
		assert_eq!(current_topics[2], winner);
	})
}
