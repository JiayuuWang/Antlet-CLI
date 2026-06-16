## 2. 经典算法：动态规划、蒙特卡洛与时序差分

强化学习的经典算法体系围绕一个核心问题展开：智能体如何在与环境的交互中学习最优策略 $\pi^*$，使累积折扣回报 $G_t = \sum_{k=0}^{\infty} \gamma^k r_{t+k+1}$ 最大化。根据对环境模型的依赖程度与数据使用方式，经典方法可分为三大类。

### 2.1 动态规划（Dynamic Programming）

动态规划（DP）假设智能体拥有完整的环境模型，即已知状态转移概率 $P(s'|s,a)$ 和奖励函数 $R(s,a)$。其理论基础是贝尔曼方程：

$$V^\pi(s) = \sum_a \pi(a|s) \sum_{s'} P(s'|s,a)\left[R(s,a,s') + \gamma V^\pi(s')\right]$$

**策略迭代（Policy Iteration）** 交替执行两步：策略评估（反复迭代直至 $V^\pi$ 收敛）与策略改进（对每个状态取贪婪动作 $\pi'(s) = \arg\max_a Q^\pi(s,a)$）。两步交替保证单调改进，最终收敛至 $\pi^*$。

**值迭代（Value Iteration）** 将策略改进内嵌于评估中，直接对贝尔曼最优方程做迭代：

$$V_{k+1}(s) = \max_a \sum_{s'} P(s'|s,a)\left[R(s,a,s') + \gamma V_k(s')\right]$$

DP 方法的瓶颈在于需要遍历全部状态空间，面对大规模或连续状态问题时计算不可行。

### 2.2 蒙特卡洛方法（Monte Carlo Methods）

蒙特卡洛（MC）方法无需环境模型，转而从完整的采样轨迹 $\tau = (s_0, a_0, r_1, s_1, \ldots, s_T)$ 中估计价值函数。其核心思想是用实际回报 $G_t$ 的样本均值近似期望值 $V(s)$。

- **首次访问 MC（First-Visit MC）**：在一条轨迹中，仅在状态 $s$ 首次出现时记录 $G_t$，用所有轨迹的首次访问回报取平均。
- **每次访问 MC（Every-Visit MC）**：对轨迹中状态 $s$ 的每次出现均记录 $G_t$，偏差更小但样本相关性更高。

MC 方法要求 episode 必须终止（有限时域），且方差较高——回报 $G_t$ 沿整条轨迹累积，随机性叠加。其优势在于无自举（bootstrapping），估计无偏。

### 2.3 时序差分学习（Temporal Difference Learning）

时序差分（TD）方法融合了 DP 的自举思想与 MC 的无模型采样，可在线、增量地更新价值估计，无需等待 episode 结束。

**TD(0) 更新规则**利用单步时序差分目标：

$$V(s_t) \leftarrow V(s_t) + \alpha \underbrace{\left[r_{t+1} + \gamma V(s_{t+1}) - V(s_t)\right]}_{\delta_t \text{（TD误差）}}$$

其中 $\alpha$ 为学习率，$\delta_t$ 称为 **TD 误差**，衡量当前估计与单步观测目标之间的差距。

#### Q-learning（Off-Policy TD）

Q-learning 直接学习最优动作价值函数 $Q^*(s,a)$，更新时使用下一状态的**最大** Q 值，与行为策略无关（off-policy）：

$$Q(s_t, a_t) \leftarrow Q(s_t, a_t) + \alpha \left[r_{t+1} + \gamma \max_{a'} Q(s_{t+1}, a') - Q(s_t, a_t)\right]$$

由于 $\max_{a'}$ 操作，Q-learning 的目标始终对应贪婪策略，因此即使使用 $\varepsilon$-贪婪等探索策略收集数据，学到的仍是最优策略。

#### SARSA（On-Policy TD）

SARSA 的名称来自其使用的五元组 $(s_t, a_t, r_{t+1}, s_{t+1}, a_{t+1})$，更新时采用**实际执行的下一动作** $a_{t+1}$（on-policy）：

$$Q(s_t, a_t) \leftarrow Q(s_t, a_t) + \alpha \left[r_{t+1} + \gamma Q(s_{t+1}, a_{t+1}) - Q(s_t, a_t)\right]$$

**核心区别**：Q-learning 评估的是贪婪策略，学习激进；SARSA 评估的是当前行为策略（含探索噪声），在危险环境中往往更保守、更安全。两者在策略收敛后行为等价，差异体现在训练过程的风险偏好上。

---

这三类方法构成了现代深度强化学习（DQN、PPO、SAC 等）的算法基础，理解其偏差-方差权衡与 on/off-policy 特性，是掌握后续 Agentic RL 复杂算法的关键前提。
