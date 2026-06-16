# 从经典强化学习到 Agentic RL

> 本文档由 Antlet 智能体集群以分形（fan-out / fan-in）方式协作生成：1 个主编 Agent 并行调度 5 个章节作者子 Agent，各自独立撰写后收割拼接而成。

## 目录

1. [强化学习基础：MDP、价值函数与策略](#1-强化学习基础mdp价值函数与策略)
2. [经典算法：动态规划、蒙特卡洛与时序差分](#2-经典算法动态规划蒙特卡洛与时序差分)
3. [深度强化学习：DQN、策略梯度与 Actor-Critic](#3-深度强化学习dqn策略梯度与-actor-critic)
4. [从 RLHF 到大模型对齐](#4-从-rlhf-到大模型对齐)
5. [Agentic RL：面向自主智能体的强化学习](#5-agentic-rl面向自主智能体的强化学习)

---

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

---

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

---

## 3. 深度强化学习：DQN、策略梯度与 Actor-Critic

深度强化学习将神经网络的表征能力与强化学习的决策框架结合，使智能体能够在高维、连续状态空间中直接从原始输入学习策略。本节梳理三条核心技术脉络：价值函数近似（DQN）、策略梯度方法（REINFORCE）与结合二者的 Actor-Critic 架构。

### 3.1 DQN：价值函数的深度近似

DQN（Deep Q-Network）由 DeepMind 于 2015 年提出，首次证明单一神经网络可在 Atari 游戏上达到人类水平。其核心是用参数为 $\theta$ 的网络 $Q(s, a; \theta)$ 近似动作价值函数，并通过最小化 Bellman 误差进行训练：

$$\mathcal{L}(\theta) = \mathbb{E}_{(s,a,r,s') \sim \mathcal{D}}\left[\left(r + \gamma \max_{a'} Q(s', a'; \theta^-) - Q(s, a; \theta)\right)^2\right]$$

DQN 引入了两项关键工程创新，解决了朴素 Q-learning 与神经网络结合时的训练不稳定问题：

**经验回放（Experience Replay）**：将智能体与环境交互产生的转移样本 $(s, a, r, s')$ 存入回放缓冲区 $\mathcal{D}$，训练时随机采样 mini-batch。这打破了时序样本间的强相关性，使梯度估计更接近 i.i.d. 假设，同时提升了数据利用效率。

**目标网络（Target Network）**：引入参数为 $\theta^-$ 的目标网络计算 TD 目标，其参数每隔固定步数从在线网络复制一次（$\theta^- \leftarrow \theta$）。这避免了优化目标随参数同步变动导致的"追逐移动靶"问题，显著稳定了训练过程。

后续工作在 DQN 基础上持续改进：Double DQN 解耦动作选择与价值估计以缓解过估计偏差，Dueling DQN 将 $Q$ 分解为状态价值 $V(s)$ 与优势函数 $A(s,a)$，优先经验回放（PER）则按 TD 误差大小对样本赋予非均匀采样概率。

### 3.2 策略梯度定理与 REINFORCE

与价值函数方法不同，策略梯度方法直接对参数化策略 $\pi_\theta(a|s)$ 进行优化。**策略梯度定理**给出了期望累积回报关于策略参数的精确梯度：

$$\nabla_\theta J(\theta) = \mathbb{E}_{\tau \sim \pi_\theta}\left[\sum_{t=0}^{T} \nabla_\theta \log \pi_\theta(a_t | s_t) \cdot G_t\right]$$

其中 $G_t = \sum_{k=t}^{T} \gamma^{k-t} r_k$ 为从时刻 $t$ 起的折扣回报。

**REINFORCE** 算法基于此定理，采用 Monte Carlo 方式用完整轨迹的实际回报估计梯度。其优点是无偏，但方差极大——尤其在 episode 较长时，$G_t$ 的方差会随时间步积累。引入**基线**（baseline）$b(s)$ 可在保持无偏性的同时降低方差：

$$\nabla_\theta J(\theta) = \mathbb{E}\left[\sum_{t} \nabla_\theta \log \pi_\theta(a_t | s_t) \cdot (G_t - b(s_t))\right]$$

常用基线为状态价值函数 $V(s_t)$，此时 $(G_t - V(s_t))$ 构成优势估计，自然地引出 Actor-Critic 框架。

### 3.3 Actor-Critic 架构

Actor-Critic 将策略（Actor）与价值函数（Critic）解耦为两个网络，以 Critic 的在线估计替代 Monte Carlo 回报，在偏差-方差之间取得平衡。

**A3C / A2C**：A3C（Asynchronous Advantage Actor-Critic）启动多个并行 Worker，各自与独立环境副本交互，异步更新共享网络参数，以多样化的经验替代经验回放。A2C 为其同步版本，等待所有 Worker 完成后统一更新，在实践中更易调试且 GPU 利用率更高。两者均以优势函数 $A(s_t, a_t) = G_t - V(s_t; \phi)$ 作为策略梯度的权重信号。

**PPO（Proximal Policy Optimization）**：PPO 是目前最广泛使用的 Actor-Critic 算法之一。其核心思想是限制每次策略更新的幅度，避免过大的参数变化破坏已学到的策略。PPO-Clip 通过裁剪概率比实现这一约束，目标函数为：

$$\mathcal{L}^{\text{CLIP}}(\theta) = \mathbb{E}_t\left[\min\left(r_t(\theta)\hat{A}_t,\ \text{clip}(r_t(\theta), 1-\epsilon, 1+\epsilon)\hat{A}_t\right)\right]$$

其中 $r_t(\theta) = \frac{\pi_\theta(a_t|s_t)}{\pi_{\theta_{\text{old}}}(a_t|s_t)}$ 为新旧策略的概率比，$\hat{A}_t$ 为优势估计，$\epsilon$（通常取 0.1~0.2）控制信任域大小。裁剪操作去掉了使目标函数在错误方向上继续增益的部分，形成保守的单调改进保证。

PPO 凭借实现简洁、超参数不敏感和良好的样本效率，成为后续 Agentic RL 研究（如 RLHF、GRPO）的重要基石。

---

## 4. 从 RLHF 到大模型对齐

大语言模型的预训练目标是预测下一个 token，这与"对人类有帮助、无害、诚实"的目标之间存在根本性的偏差。**基于人类反馈的强化学习**（Reinforcement Learning from Human Feedback，RLHF）是目前主流的对齐范式，通过将人类偏好信号注入训练过程，引导模型行为向期望方向收敛。

### 4.1 RLHF 三阶段流程

**阶段一：监督微调（SFT）**

在精心标注的高质量示范数据上对预训练模型进行有监督微调，得到初始策略 $\pi^{\text{SFT}}$。SFT 阶段确立了模型遵循指令的基本能力，是后续强化学习的起点。

**阶段二：奖励模型训练（Reward Model）**

收集人类对模型输出的成对偏好数据 $(x, y_w, y_l)$，其中 $y_w$ 为偏好回答，$y_l$ 为非偏好回答。采用 **Bradley-Terry 模型**对偏好概率建模：

$$
P(y_w \succ y_l \mid x) = \frac{\exp\, r(x, y_w)}{\exp\, r(x, y_w) + \exp\, r(x, y_l)} = \sigma\!\left(r(x, y_w) - r(x, y_l)\right)
$$

其中 $r(x, y)$ 为奖励模型打分，$\sigma$ 为 sigmoid 函数。训练目标为最大化对数似然：

$$
\mathcal{L}_{\text{RM}} = -\mathbb{E}_{(x,\, y_w,\, y_l)}\!\left[\log \sigma\!\left(r(x, y_w) - r(x, y_l)\right)\right]
$$

**阶段三：PPO 强化学习微调**

以训练好的奖励模型 $r_\phi$ 为环境信号，使用 PPO 算法优化策略 $\pi_\theta$。为防止策略偏离 SFT 基线过远（奖励黑客问题），目标函数引入 **KL 散度惩罚**：

$$
\mathcal{J}(\theta) = \mathbb{E}_{x \sim \mathcal{D},\, y \sim \pi_\theta(\cdot|x)}\!\left[r_\phi(x, y) - \beta\, \mathbb{KL}\!\left[\pi_\theta(\cdot|x) \,\|\, \pi^{\text{SFT}}(\cdot|x)\right]\right]
$$

KL 约束系数 $\beta$ 控制对齐强度与能力保留之间的权衡：$\beta$ 过小，模型易出现奖励黑客；$\beta$ 过大，优化空间受限，对齐效果不足。

### 4.2 DPO：无需奖励模型的直接偏好优化

RLHF 流程复杂，奖励模型的训练误差会在 PPO 阶段被放大。**Direct Preference Optimization（DPO）** 通过数学变换，将奖励模型隐式嵌入策略本身，直接从偏好数据优化语言模型，绕过了显式 RM 的训练。

其核心洞察是：在 KL 约束的 RL 目标下，最优策略与奖励函数之间存在解析关系：

$$
r(x, y) = \beta \log \frac{\pi^*(y|x)}{\pi^{\text{ref}}(y|x)} + \beta \log Z(x)
$$

将此关系代入 Bradley-Terry 模型，DPO 的训练目标简化为：

$$
\mathcal{L}_{\text{DPO}}(\theta) = -\mathbb{E}\!\left[\log \sigma\!\left(\beta \log \frac{\pi_\theta(y_w|x)}{\pi_{\text{ref}}(y_w|x)} - \beta \log \frac{\pi_\theta(y_l|x)}{\pi_{\text{ref}}(y_l|x)}\right)\right]
$$

DPO 将对齐问题转化为一个简单的分类损失，无需强化学习采样循环，训练稳定性显著提升，工程实现成本大幅降低。

### 4.3 对齐税

对齐并非没有代价。经过 RLHF 或 DPO 微调的模型，在部分基准测试（如代码生成、数学推理）上往往出现性能下滑，这一现象被称为**对齐税（alignment tax）**。其成因主要有两点：

1. **分布偏移**：SFT 和偏好数据的分布与预训练语料存在差异，微调可能覆盖部分知识。
2. **目标冲突**：奖励模型倾向于奖励"听起来正确"的回答，而非实际正确的推理链，导致模型在形式上对齐但能力退化。

缓解对齐税的方向包括：增大 SFT 数据质量与多样性、采用宪法 AI（CAI）减少人工标注噪声、以及在 RLHF 目标中混入预训练损失（即 PPO-ptx）。

RLHF 及其衍生方法奠定了现代对齐技术的基础，也为后续 Agentic RL 中更复杂的奖励设计与多步决策优化提供了理论依据。

---

## 5. Agentic RL：面向自主智能体的强化学习

Agentic RL 是经典强化学习在大语言模型（LLM）时代的自然延伸。与传统 RL 智能体在有限状态空间中追求累积奖励不同，Agentic RL 的目标是赋予模型在开放环境中**自主规划、调用工具、并多步完成复杂任务**的能力。

### 5.1 核心挑战

**长程规划（Long-horizon Planning）**  
现实任务往往需要数十乃至数百步的决策序列。传统 RL 的回报折扣机制难以有效传播远端信号：当折扣因子 $\gamma \to 1$ 时，方差爆炸；当 $\gamma \ll 1$ 时，远端目标被遗忘。Agentic RL 需要在语言空间中维护跨步骤的任务状态，这对上下文窗口和记忆机制提出了严苛要求。

**稀疏奖励（Sparse Reward）**  
在代码生成、科学推理等任务中，奖励信号往往只在任务终点出现（如测试通过与否）。策略梯度估计器的方差因此急剧增大：

$$\nabla_\theta J(\theta) = \mathbb{E}_{\tau \sim \pi_\theta}\left[\sum_{t=0}^{T} \nabla_\theta \log \pi_\theta(a_t|s_t) \cdot G_t\right]$$

当 $G_t$ 极度稀疏时，大量采样轨迹的梯度为零，训练效率极低。奖励塑形（reward shaping）与课程学习（curriculum learning）是常用的缓解手段。

**工具使用（Tool Use）**  
Agentic 系统需要动态决定何时调用外部工具（搜索引擎、代码解释器、数据库）。工具调用引入了离散动作空间与非可微的外部反馈，使得标准策略梯度方法需要适配为混合动作空间的优化框架。

---

### 5.2 思维链与过程奖励模型

**过程奖励模型（PRM）vs 结果奖励模型（ORM）**  
ORM 仅对最终答案打分，信号稀疏；PRM 对推理链中的每一步打分，提供密集的中间监督：

$$r_{\text{PRM}}(s_t, a_t) = f_\phi(s_0, a_0, \ldots, s_t, a_t)$$

实验表明，PRM 能显著提升数学推理和多步问答任务的策略学习效率。其核心挑战在于**如何标注中间步骤的质量**——近期工作（如 MATH-Shepherd）探索了通过蒙特卡洛采样自动估计过程奖励的方法，无需人工逐步标注。

思维链（Chain-of-Thought, CoT）本质上是将隐式的推理过程显式化为语言 token 序列，使得 PRM 可以直接在自然语言推理步骤上施加奖励信号，二者形成天然的协同。

---

### 5.3 ReAct 与 Reflexion 框架

**ReAct**（Reason + Act）将推理轨迹与行动交替生成：智能体先输出思考（`Thought`），再执行动作（`Action`），最后观测环境反馈（`Observation`），形成 $(T, A, O)$ 循环。这一结构使工具调用时机与推理过程显式绑定，可解释性强。

**Reflexion** 在 ReAct 基础上引入了**语言反思**机制：智能体在每次轨迹结束后，由 LLM 生成对失败原因的自然语言总结，并将其存入情景记忆（episodic memory）。在后续回合中，智能体可检索历史反思以规避重复错误，相当于在语言空间中实现了一种轻量级的策略更新，无需梯度回传。

---

### 5.4 多智能体强化学习基础

多智能体系统（MARL）将单智能体的 MDP 扩展为**去中心化部分可观测马尔可夫博弈**（Dec-POMDP）。每个智能体 $i$ 在联合状态 $s$ 下依据局部观测 $o^i$ 选择动作 $a^i$，联合奖励为 $r(s, \mathbf{a})$。

在 Agentic LLM 场景中，多智能体范式常见于：
- **角色分工**：规划者（Planner）+ 执行者（Executor）+ 评审者（Critic）
- **辩论与对抗**：多个 LLM 实例互相质疑以提升答案可靠性
- **自我博弈（Self-play）**：智能体与自身历史版本对抗，持续提升上限

---

### 5.5 未来方向

| 方向 | 核心思想 | 挑战 |
|------|----------|------|
| **世界模型（World Model）** | 学习环境的内部表征 $\hat{s}_{t+1} = f(s_t, a_t)$，在想象空间中规划 | 分布外泛化，幻觉累积 |
| **自我博弈（Self-play）** | 通过与自身对弈产生课程，突破人类标注上限 | 模式崩塌，奖励欺骗 |
| **持续学习（Continual Learning）** | 在不遗忘旧技能的前提下习得新任务 | 灾难性遗忘，任务边界模糊 |

Agentic RL 的终极目标，是构建能够在真实世界中**自主设定子目标、验证假设、修正策略**的通用智能体。这要求将经典 RL 的数学严谨性与 LLM 的语言泛化能力深度融合，是当前 AI 研究最具挑战性的前沿之一。
