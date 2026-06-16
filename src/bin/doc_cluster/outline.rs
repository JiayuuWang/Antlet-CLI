//! Deterministic outline generator for the RL → Agentic RL technical document.
//!
//! Produces a deep tree (Part → Chapter → Section → Module). Each leaf
//! `Module` is one unit of work assigned to exactly one agent instance.
//! With ~1350 modules at ~780 words each the assembled document exceeds
//! one million Chinese characters.

#[derive(Clone)]
pub struct Module {
    pub id: String,         // e.g. "P1.C2.S3.M4"
    pub part: String,       // part title
    pub chapter: String,    // chapter title
    pub section: String,    // section title
    pub title: String,      // module title
    pub points: Vec<String>,// writing points / sub-topics to cover
    pub target_words: u32,  // target word count for this module
}

struct PartSpec {
    title: &'static str,
    chapters: Vec<ChapterSpec>,
}
struct ChapterSpec {
    title: &'static str,
    sections: Vec<SectionSpec>,
}
struct SectionSpec {
    title: &'static str,
    /// Module topics; each becomes one leaf assigned to one agent.
    modules: Vec<&'static str>,
}

/// Build the full outline tree and flatten it into a list of modules.
///
/// Each curriculum topic is split into several complementary modules (facets)
/// so the document fans out to enough independent work units to exceed one
/// million Chinese characters. Each facet is written by its own agent instance.
pub fn build_outline(target_words_per_module: u32) -> Vec<Module> {
    let parts = course();
    let mut out = Vec::new();
    for (pi, p) in parts.iter().enumerate() {
        for (ci, c) in p.chapters.iter().enumerate() {
            for (si, s) in c.sections.iter().enumerate() {
                for (mi, topic) in s.modules.iter().enumerate() {
                    for (fi, facet) in FACETS.iter().enumerate() {
                        let id = format!(
                            "P{}.C{}.S{}.M{}.F{}",
                            pi + 1, ci + 1, si + 1, mi + 1, fi + 1
                        );
                        let title = format!("{}：{}", topic, facet.suffix);
                        out.push(Module {
                            id,
                            part: p.title.to_string(),
                            chapter: c.title.to_string(),
                            section: s.title.to_string(),
                            title,
                            points: (facet.points)(topic),
                            target_words: target_words_per_module,
                        });
                    }
                }
            }
        }
    }
    out
}

/// A facet is one angle on a topic, turned into a standalone module.
struct Facet {
    suffix: &'static str,
    points: fn(&str) -> Vec<String>,
}

const FACETS: &[Facet] = &[
    Facet {
        suffix: "概念与动机",
        points: facet_concept,
    },
    Facet {
        suffix: "原理与推导",
        points: facet_theory,
    },
    Facet {
        suffix: "算法与实践",
        points: facet_practice,
    },
];

fn facet_concept(topic: &str) -> Vec<String> {
    vec![
        format!("精确定义「{}」的核心概念、术语与符号", topic),
        format!("阐述「{}」要解决的问题与提出动机", topic),
        "给出直观解释与必要的形式化定义（含 LaTeX 公式）".to_string(),
        "说明其在强化学习/Agentic RL 知识体系中的位置".to_string(),
        "指出适用场景与基本假设".to_string(),
    ]
}

fn facet_theory(topic: &str) -> Vec<String> {
    vec![
        format!("给出「{}」的关键数学公式与定理（使用 LaTeX）", topic),
        "推导或论证其核心结论的来由".to_string(),
        "分析其性质：收敛性、偏差-方差、复杂度等".to_string(),
        format!("讨论「{}」与相邻方法的理论联系与区别", topic),
        "指出理论上的局限与边界条件".to_string(),
    ]
}

fn facet_practice(topic: &str) -> Vec<String> {
    vec![
        format!("给出「{}」的代表性算法流程或伪代码描述", topic),
        "说明关键实现细节、超参数与工程要点".to_string(),
        "列举典型应用、实验结论或基准表现".to_string(),
        "分析常见失败模式与调优经验".to_string(),
        "给出与其他方案的实践对比与选型建议".to_string(),
    ]
}

fn s(title: &'static str, modules: Vec<&'static str>) -> SectionSpec {
    SectionSpec { title, modules }
}
fn c(title: &'static str, sections: Vec<SectionSpec>) -> ChapterSpec {
    ChapterSpec { title, sections }
}

/// The full curriculum. Designed to fan out to ~1350 modules.
fn course() -> Vec<PartSpec> {
    vec![
        PartSpec {
            title: "第一篇 强化学习的数学基础",
            chapters: vec![
                c("第1章 序贯决策与马尔可夫决策过程", vec![
                    s("1.1 序贯决策问题", vec![
                        "序贯决策的基本范式","智能体-环境交互回路","奖励假设与目标","回合制与持续性任务","部分可观测性引入"]),
                    s("1.2 马尔可夫决策过程 MDP", vec![
                        "马尔可夫性质","MDP 五元组定义","状态空间与动作空间","状态转移概率","奖励函数的形式","折扣因子与折扣回报","有限与无限时域 MDP"]),
                    s("1.3 策略与回报", vec![
                        "确定性策略","随机性策略","轨迹与回报定义","期望回报","平稳分布与遍历性"]),
                    s("1.4 价值函数", vec![
                        "状态价值函数 V","动作价值函数 Q","优势函数 A","贝尔曼期望方程","贝尔曼最优方程","最优价值与最优策略","价值函数的存在唯一性"]),
                ]),
                c("第2章 动态规划方法", vec![
                    s("2.1 策略评估", vec![
                        "迭代策略评估","收敛性证明思路","就地更新与同步更新"]),
                    s("2.2 策略改进与控制", vec![
                        "策略改进定理","策略迭代算法","值迭代算法","广义策略迭代","异步动态规划"]),
                    s("2.3 动态规划的性质", vec![
                        "压缩映射与不动点","收敛速率分析","维数灾难","近似动态规划引入"]),
                ]),
                c("第3章 蒙特卡洛方法", vec![
                    s("3.1 蒙特卡洛预测", vec![
                        "首次访问蒙特卡洛","每次访问蒙特卡洛","增量式均值更新","偏差与方差权衡"]),
                    s("3.2 蒙特卡洛控制", vec![
                        "探索性起始","同轨策略蒙特卡洛控制","重要性采样","离轨策略蒙特卡洛","加权重要性采样"]),
                ]),
                c("第4章 时序差分学习", vec![
                    s("4.1 TD 预测", vec![
                        "TD(0) 更新规则","TD 误差","TD 与蒙特卡洛比较","TD 的收敛性"]),
                    s("4.2 TD 控制", vec![
                        "SARSA 同轨控制","Q-learning 离轨控制","期望 SARSA","Double Q-learning","最大化偏差问题"]),
                    s("4.3 多步与资格迹", vec![
                        "n 步 TD","λ 回报","TD(λ) 算法","前向视角与后向视角","资格迹的实现"]),
                ]),
            ],
        },
        PartSpec {
            title: "第二篇 函数逼近与深度强化学习",
            chapters: vec![
                c("第5章 值函数逼近", vec![
                    s("5.1 逼近的动机与框架", vec![
                        "表格方法的局限","线性函数逼近","特征构造与瓦片编码","随机梯度下降目标"]),
                    s("5.2 离轨逼近的稳定性", vec![
                        "致命三要素","Baird 反例","梯度 TD 方法","Emphatic TD"]),
                ]),
                c("第6章 深度 Q 网络", vec![
                    s("6.1 DQN 基础", vec![
                        "深度网络逼近 Q","经验回放","目标网络","DQN 训练流程","Atari 基准"]),
                    s("6.2 DQN 改进族", vec![
                        "Double DQN","Dueling DQN","优先经验回放","Noisy Nets","多步 DQN","分布式 DQN C51","Rainbow 集成"]),
                ]),
                c("第7章 策略梯度方法", vec![
                    s("7.1 策略梯度基础", vec![
                        "策略参数化","策略梯度定理","REINFORCE 算法","基线与方差缩减","因果性与奖励-to-go"]),
                    s("7.2 Actor-Critic", vec![
                        "Actor-Critic 架构","优势 Actor-Critic A2C","异步 A3C","广义优势估计 GAE","熵正则化"]),
                    s("7.3 信赖域与近端方法", vec![
                        "自然策略梯度","TRPO 信赖域","重要性比与代理目标","PPO 裁剪目标","PPO 实现要点"]),
                ]),
                c("第8章 连续控制与确定性策略", vec![
                    s("8.1 确定性策略梯度", vec![
                        "DPG 定理","DDPG 算法","TD3 三大改进","软更新与探索噪声"]),
                    s("8.2 最大熵强化学习", vec![
                        "最大熵目标","软价值函数","Soft Actor-Critic SAC","温度自动调节"]),
                ]),
                c("第9章 模型基础强化学习", vec![
                    s("9.1 模型学习与规划", vec![
                        "环境模型的类型","Dyna 架构","模型预测控制 MPC","蒙特卡洛树搜索 MCTS"]),
                    s("9.2 现代基于模型方法", vec![
                        "世界模型 World Models","PlaNet 与隐空间规划","Dreamer 系列","MuZero 隐式模型"]),
                ]),
            ],
        },
        PartSpec {
            title: "第三篇 探索、泛化与高级主题",
            chapters: vec![
                c("第10章 探索与利用", vec![
                    s("10.1 经典探索策略", vec![
                        "ε-贪婪","乐观初始化","UCB 上置信界","玻尔兹曼探索"]),
                    s("10.2 深度探索", vec![
                        "计数与伪计数","内在好奇心模块 ICM","随机网络蒸馏 RND","信息增益与贝叶斯探索"]),
                ]),
                c("第11章 分层与时序抽象", vec![
                    s("11.1 选项框架", vec![
                        "半马尔可夫决策过程","选项与内部策略","选项-评论家"]),
                    s("11.2 目标条件与分层", vec![
                        "目标条件强化学习","HER 事后经验回放","封建网络 FeUdal","分层策略学习"]),
                ]),
                c("第12章 离线强化学习", vec![
                    s("12.1 离线 RL 的挑战", vec![
                        "分布偏移","外推误差","行为正则化"]),
                    s("12.2 代表性离线算法", vec![
                        "BCQ","CQL 保守 Q 学习","IQL 隐式 Q 学习","决策 Transformer"]),
                ]),
                c("第13章 多智能体强化学习", vec![
                    s("13.1 博弈论基础", vec![
                        "随机博弈","纳什均衡","合作与竞争设置","Dec-POMDP"]),
                    s("13.2 MARL 算法", vec![
                        "独立 Q 学习","集中训练分散执行 CTDE","QMIX 值分解","MADDPG","自我博弈与种群训练"]),
                ]),
                c("第14章 模仿学习与逆强化学习", vec![
                    s("14.1 模仿学习", vec![
                        "行为克隆","DAgger","分布漂移问题"]),
                    s("14.2 逆强化学习", vec![
                        "最大熵 IRL","对抗模仿 GAIL","奖励塑形与势函数"]),
                ]),
            ],
        },
        PartSpec {
            title: "第四篇 大模型与人类对齐",
            chapters: vec![
                c("第15章 预训练语言模型基础", vec![
                    s("15.1 Transformer 与自回归建模", vec![
                        "自注意力机制","自回归语言建模目标","缩放定律","上下文学习能力"]),
                    s("15.2 指令微调", vec![
                        "监督微调 SFT","指令数据构造","多任务指令泛化"]),
                ]),
                c("第16章 基于人类反馈的强化学习 RLHF", vec![
                    s("16.1 偏好建模", vec![
                        "成对偏好数据","Bradley-Terry 模型","奖励模型训练","奖励模型的校准"]),
                    s("16.2 RLHF 流程", vec![
                        "三阶段流程总览","PPO 微调语言模型","KL 散度约束","奖励黑客问题","PPO-ptx 混合目标"]),
                ]),
                c("第17章 免强化学习的对齐方法", vec![
                    s("17.1 直接偏好优化", vec![
                        "DPO 推导","DPO 损失函数","隐式奖励视角","IPO 与 cDPO 变体"]),
                    s("17.2 其他对齐范式", vec![
                        "RLAIF 与 AI 反馈","宪法 AI","拒绝采样微调","KTO 与二元反馈"]),
                ]),
                c("第18章 对齐的深层问题", vec![
                    s("18.1 对齐税与能力保持", vec![
                        "对齐税现象","遗忘与能力退化","多目标权衡"]),
                    s("18.2 可扩展监督", vec![
                        "弱到强泛化","辩论与递归奖励建模","过程监督 vs 结果监督"]),
                ]),
            ],
        },
        PartSpec {
            title: "第五篇 推理与智能体强化学习",
            chapters: vec![
                c("第19章 大模型推理能力", vec![
                    s("19.1 思维链推理", vec![
                        "思维链提示","自洽性解码","思维树 ToT","推理时计算扩展"]),
                    s("19.2 推理的强化学习", vec![
                        "结果奖励模型 ORM","过程奖励模型 PRM","自动过程标注","可验证奖励 RLVR","GRPO 群体相对策略优化"]),
                ]),
                c("第20章 智能体强化学习 Agentic RL", vec![
                    s("20.1 智能体范式", vec![
                        "智能体的定义与构成","感知-决策-行动循环","工具使用与函数调用","记忆与上下文管理"]),
                    s("20.2 智能体框架", vec![
                        "ReAct 推理与行动","Reflexion 语言反思","Plan-and-Solve","自我批判与修正"]),
                    s("20.3 Agentic RL 的训练", vec![
                        "长程信用分配","稀疏与延迟奖励","轨迹级优化","工具调用的策略学习","环境反馈作为奖励"]),
                ]),
                c("第21章 多智能体与协作系统", vec![
                    s("21.1 多智能体协作", vec![
                        "角色分工架构","规划者-执行者-评审者","多智能体辩论","通信协议学习"]),
                    s("21.2 自我提升", vec![
                        "自我博弈生成课程","拒绝采样自训练","合成数据飞轮","持续学习与灾难性遗忘"]),
                ]),
                c("第22章 前沿与展望", vec![
                    s("22.1 关键挑战", vec![
                        "奖励设计与规格博弈","安全与可控性","泛化与鲁棒性","评测与基准"]),
                    s("22.2 未来方向", vec![
                        "世界模型与想象规划","具身智能体","终身学习智能体","通用智能体的路径"]),
                ]),
            ],
        },
        PartSpec {
            title: "第六篇 实践、系统与案例",
            chapters: vec![
                c("第23章 强化学习工程实践", vec![
                    s("23.1 实现要点", vec![
                        "环境接口与 Gym 规范","向量化环境","观测归一化","奖励缩放","随机种子与可复现性"]),
                    s("23.2 训练稳定性", vec![
                        "超参数调优","梯度裁剪","学习率调度","训练诊断与可视化"]),
                    s("23.3 分布式训练", vec![
                        "Actor-Learner 架构","IMPALA 与 V-trace","Ape-X 分布式回放","大规模并行采样"]),
                ]),
                c("第24章 大模型 RL 训练系统", vec![
                    s("24.1 训练基础设施", vec![
                        "PPO 训练管线","经验生成与打分","显存优化与并行","推理引擎集成"]),
                    s("24.2 开源框架", vec![
                        "TRL 库","DeepSpeed-Chat","OpenRLHF","veRL 与 HybridFlow"]),
                ]),
                c("第25章 典型案例研究", vec![
                    s("25.1 游戏与控制", vec![
                        "AlphaGo 与 AlphaZero","AlphaStar 星际争霸","OpenAI Five Dota","机器人操作学习"]),
                    s("25.2 大模型对齐案例", vec![
                        "InstructGPT","Claude 与宪法 AI","DeepSeek-R1 推理模型","开源对齐模型实践"]),
                    s("25.3 智能体应用", vec![
                        "代码智能体","计算机操作智能体","科学发现智能体","Web 浏览智能体"]),
                ]),
            ],
        },
    ]
}
