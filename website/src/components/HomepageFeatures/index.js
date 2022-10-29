import React from 'react';
import clsx from 'clsx';
import styles from './styles.module.css';

const FeatureList = [
  {
    title: 'Flexible',
    description: (
      <>
        Your use case should be achievable without manually sending and receiving packets or other hacks.
      </>
    ),
  },
  {
    title: 'Minimal',
    description: (
      <>
        The API surface is small with only the necessities exposed. Opinionated features such as a standalone executable, plugin system, and reimplementation of vanilla mechanics should be built in a separate crate on top of the foundation that Valence provides.
      </>
    ),
  },
  {
    title: 'Intuitive',
    description: (
      <>
        An API that is easy to use and difficult to misuse. Extensive documentation is important.
      </>
    ),
  },
];

function Feature({title, description}) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">
      </div>
      <div className="text--center padding-horiz--md">
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
