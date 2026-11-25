import re
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
from tools import parity


class _FakeExecutor:
    """Scripted executor: returns a canned status for each (label, reuse) pair."""
    dead = False
    max_peak = None
    name = 'fake'

    def __init__(self, plan):
        self.plan = plan
        self.pending = []
        self.submits = []

    def free(self):
        return 8

    def accepts(self, name):
        return True

    def fits(self, expected, only_here=True):
        return True

    def start(self, name, script, reuse, expected, shard=None):
        self.submits.append((name, reuse))
        self.pending.append((name, reuse))
        return True

    def poll(self):
        finished = [(label, {'status': self.plan[(label, reuse)],
                             'seconds': 1, 'dependencies': []})
                    for label, reuse in self.pending]
        self.pending.clear()
        return finished


def _drive(rounds, plan):
    ex = _FakeExecutor(plan)
    state = {'harnesses': {'h': {'seconds': 1, 'oracle_reuse': True, 'status': 'pass'}}}
    with mock.patch.object(parity, 'save_state'), mock.patch.object(parity, 'publish'):
        results, _ = parity.run(
            {'h': 'tools/test_h_parity.py'}, state,
            hasher=None, environment_digest='env', executors=[ex],
            reuse_oracles=True, rounds=rounds, known_flaky={},
        )
    return results['h'], ex.submits


class ParityRetryTests(unittest.TestCase):
    def test_reused_oracle_marker_survives_multi_round_retry(self):
        # One-round control: a fresh retry after a reused-oracle failure keeps the marker.
        one, _ = _drive({'h': 1}, {('h', True): 'fail', ('h', False): 'pass'})
        self.assertEqual(one.get('retried'), 'reused oracle results')

        # Two rounds: round 1 is retried fresh, so the final round's label is 'h~2'.
        two, submits = _drive(
            {'h': 2},
            {('h', True): 'fail', ('h', False): 'pass', ('h~2', True): 'pass'},
        )
        self.assertIn(('h', False), submits, 'round 1 must have been retried fresh')
        self.assertIs(two.get('oracle_reuse'), False)
        self.assertEqual(two.get('retried'), 'reused oracle results')




if __name__ == '__main__':
    unittest.main()
