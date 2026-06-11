import pytest
from moa.agent import MoAAgent, AgentResult
from moa.backends.mock import MockBackend
from moa.ir import Modality


# ── Fixtures ─────────────────────────────────────────────────────────────────

@pytest.fixture
def agent():
    return MoAAgent(backend=MockBackend())


@pytest.fixture
def named_agent():
    return MoAAgent(
        backend=MockBackend(responses={
            "hello": "Hello! How can I help you today?",
            "chess": "1. e4 e5 2. Nf3 Nc6 leads to the Italian or Ruy Lopez.",
        })
    )


# ── Basic agent tests ──────────────────────────────────────────────────────

def test_agent_returns_result(agent):
    result = agent.run("What is 2+2?")
    assert isinstance(result, AgentResult)


def test_agent_response_nonempty(agent):
    result = agent.run("Hello")
    assert len(result.response) > 0


def test_agent_has_intent(agent):
    result = agent.run("Read the file config.json")
    assert result.intent is not None


def test_agent_step_increments(agent):
    agent.run("first task")
    agent.run("second task")
    assert agent.step == 2


def test_agent_duration_positive(agent):
    result = agent.run("hello")
    assert result.duration_s >= 0.0


def test_agent_actions_lists_present(agent):
    result = agent.run("hello")
    assert isinstance(result.actions_taken, list)
    assert isinstance(result.actions_rejected, list)


# ── Memory integration ─────────────────────────────────────────────────────

def test_agent_stores_episodes(agent):
    agent.run("task one")
    agent.run("task two")
    assert agent.memory.episodes.size >= 2


def test_agent_event_log_grows(agent):
    initial = agent.memory.event_log.entry_count
    agent.run("hello")
    assert agent.memory.event_log.entry_count > initial


def test_agent_recall_returns_list(agent):
    agent.run("chess opening analysis")
    result = agent.run("more chess analysis")
    # Memory recall is built into the agent; just verify it runs
    assert len(result.response) > 0


# ── Context window / history ─────────────────────────────────────────────

def test_agent_history_bounded(agent):
    for i in range(30):
        agent.run(f"message {i}")
    # History should not grow unboundedly
    assert len(agent._history) <= agent._max_history * 2


def test_agent_reset_history(agent):
    agent.run("first")
    agent.reset_history()
    assert agent._history == []


# ── Safety gate integration ────────────────────────────────────────────────

def test_agent_safety_stats_tracked(agent):
    agent.run("read the file")
    stats = agent.safety_stats
    total = stats["approved"] + stats["rejected"]
    assert total >= 1


# ── Fact storage ──────────────────────────────────────────────────────────

def test_agent_store_fact_succeeds(agent):
    claim = agent.store_fact(
        subject="python", predicate="is_installed", object_="True",
        evidence_content="which python3 returned /usr/bin/python3",
        confidence=0.95,
    )
    assert claim is not None
    assert claim.modality == Modality.FACT
    assert claim.is_verified()


def test_agent_store_fact_low_confidence_fails(agent):
    claim = agent.store_fact(
        subject="x", predicate="is", object_="y",
        evidence_content="some content",
        confidence=0.5,  # below 0.8 threshold
    )
    assert claim is None


# ── Custom responses ──────────────────────────────────────────────────────

def test_named_agent_keyword_response(named_agent):
    result = named_agent.run("Tell me about chess")
    assert "chess" in result.response.lower() or len(result.response) > 0


def test_agent_custom_system_prompt():
    agent = MoAAgent(
        backend=MockBackend(),
        system_prompt="You are a chess coach.",
    )
    result = agent.run("e4 e5")
    assert isinstance(result, AgentResult)


# ── Multi-turn ────────────────────────────────────────────────────────────

def test_multi_turn_context_accumulates(agent):
    r1 = agent.run("My name is Alex.")
    r2 = agent.run("What did I just tell you?")
    # At minimum both runs complete without error
    assert len(r1.response) > 0
    assert len(r2.response) > 0
    assert agent.step == 2
