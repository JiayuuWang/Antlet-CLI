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
