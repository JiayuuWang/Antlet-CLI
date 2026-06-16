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
