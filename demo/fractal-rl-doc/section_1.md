## 1. 强化学习基础：MDP、价值函数与策略

强化学习（Reinforcement Learning, RL）的核心数学框架是**马尔可夫决策过程**（Markov Decision Process, MDP）。它将序列决策问题形式化，为后续的算法设计提供了统一的语言。

### 1.1 马尔可夫决策过程

一个 MDP 由五元组 $\langle \mathcal{S}, \mathcal{A}, P, R, \gamma \rangle$ 定义：

- **状态空间** $\mathcal{S}$：环境所有可能状态的集合。$s_t \in \mathcal{S}$ 表示智能体在时刻 $t$ 观察到的环境状态。
- **动作空间** $\mathcal{A}$：智能体可执行的所有动作的集合。$a_t \in \mathcal{A}$ 表示在状态 $s_t$ 下所采取的动作。
- **转移概率** $P$：环境的动态模型，$P(s' \mid s, a)$ 表示在状态 $s$ 执行动作 $a$ 后转移到状态 $s'$ 的概率。MDP 满足**马尔可夫性质**：下一状态仅依赖于当前状态与动作，与历史无关。
- **奖励函数** $R$：$R(s, a, s')$ 表示从状态 $s$ 执行动作 $a$ 转移到 $s'$ 时获得的即时标量奖励信号。
- **折扣因子** $\gamma \in [0, 1)$：对未来奖励进行指数衰减，使累积回报收敛。折扣回报定义为：

$$G_t = \sum_{k=0}^{\infty} \gamma^k R_{t+k+1}$$

$\gamma$ 越接近 1，智能体越"有远见"；越接近 0，则越短视，只关注即时奖励。

### 1.2 价值函数与贝尔曼方程

价值函数量化了智能体在给定状态（或状态-动作对）下的长期收益预期，是 RL 算法的核心量。

**状态价值函数** $V^\pi(s)$ 表示从状态 $s$ 出发、遵循策略 $\pi$ 所能获得的期望回报：

$$V^\pi(s) = \mathbb{E}_\pi \left[ G_t \mid s_t = s \right] = \mathbb{E}_\pi \left[ R_{t+1} + \gamma V^\pi(s_{t+1}) \mid s_t = s \right]$$

**动作价值函数** $Q^\pi(s, a)$ 则进一步条件化于当前动作：

$$Q^\pi(s, a) = \mathbb{E}_\pi \left[ G_t \mid s_t = s, a_t = a \right] = \mathbb{E}_\pi \left[ R_{t+1} + \gamma Q^\pi(s_{t+1}, a_{t+1}) \mid s_t = s, a_t = a \right]$$

两者通过策略 $\pi$ 相互关联：$V^\pi(s) = \sum_a \pi(a \mid s)\, Q^\pi(s, a)$。

上述递推形式即**贝尔曼期望方程**（Bellman Expectation Equation）。在最优策略 $\pi^*$ 下，贝尔曼最优方程变为：

$$Q^*(s, a) = \mathbb{E}\left[ R_{t+1} + \gamma \max_{a'} Q^*(s_{t+1}, a') \mid s_t = s, a_t = a \right]$$

该方程是 Q-learning、DQN 等值函数方法的理论基础。

### 1.3 策略的定义与分类

**策略** $\pi$ 是从状态到动作（或动作分布）的映射，决定了智能体的行为方式。根据输出形式，策略分为两类：

- **确定性策略**（Deterministic Policy）：$\pi: \mathcal{S} \to \mathcal{A}$，在每个状态下输出唯一确定的动作 $a = \pi(s)$。DDPG、TD3 等算法采用此形式，适合连续动作空间。

- **随机策略**（Stochastic Policy）：$\pi: \mathcal{S} \to \Delta(\mathcal{A})$，输出动作上的概率分布 $\pi(a \mid s)$。策略梯度方法（REINFORCE、PPO、SAC）均基于随机策略，其内在的随机性有助于探索与熵正则化。

两类策略在理论上等价——最优确定性策略一定存在（在有限 MDP 中），但随机策略在部分可观测、多智能体或需要探索的场景中更具优势。

MDP 框架及其衍生的价值函数与策略概念，构成了从经典表格方法到深度强化学习、再到 Agentic RL 的共同理论基石。
