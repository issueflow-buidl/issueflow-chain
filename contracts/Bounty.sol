// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract Bounty {
    address public owner;

    struct BountyInfo {
        address creator;
        address token;
        uint256 amount;
        uint256 deadline;
        bool claimed;
        bool cancelled;
    }

    uint256 public nextBountyId;
    mapping(uint256 => BountyInfo) public bounties;

    event BountyCreated(uint256 indexed bountyId, address indexed creator, address token, uint256 amount, uint256 deadline);
    event BountyClaimed(uint256 indexed bountyId, address indexed solver, uint256 amount);
    event BountyCancelled(uint256 indexed bountyId, uint256 amount);

    constructor() {
        owner = msg.sender;
    }

    function createBounty(address token, uint256 amount, uint256 deadline) external returns (uint256) {
        require(amount > 0, "amount must be positive");
        require(deadline > block.timestamp, "deadline must be in the future");

        uint256 bountyId = nextBountyId++;
        bounties[bountyId] = BountyInfo({
            creator: msg.sender,
            token: token,
            amount: amount,
            deadline: deadline,
            claimed: false,
            cancelled: false
        });

        // Transfer tokens from creator to this contract (escrow).
        // Note: requires prior ERC-20 approve from msg.sender.
        require(IERC20(token).transferFrom(msg.sender, address(this), amount), "transfer failed");

        emit BountyCreated(bountyId, msg.sender, token, amount, deadline);
        return bountyId;
    }

    function claimBounty(uint256 bountyId, address solver) external {
        BountyInfo storage bounty = bounties[bountyId];
        require(bounty.creator != address(0), "bounty not found");
        require(!bounty.claimed && !bounty.cancelled, "bounty not open");
        require(bounty.creator == msg.sender, "only creator can claim");

        bounty.claimed = true;
        require(IERC20(bounty.token).transfer(solver, bounty.amount), "transfer failed");

        emit BountyClaimed(bountyId, solver, bounty.amount);
    }

    function cancelBounty(uint256 bountyId) external {
        BountyInfo storage bounty = bounties[bountyId];
        require(bounty.creator != address(0), "bounty not found");
        require(!bounty.claimed && !bounty.cancelled, "bounty not open");
        require(bounty.creator == msg.sender, "only creator can cancel");
        require(block.timestamp >= bounty.deadline, "deadline not passed");

        bounty.cancelled = true;
        require(IERC20(bounty.token).transfer(msg.sender, bounty.amount), "transfer failed");

        emit BountyCancelled(bountyId, bounty.amount);
    }
}

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function transfer(address to, uint256 amount) external returns (bool);
}
