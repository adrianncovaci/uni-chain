import React, { useEffect, useState } from 'react'
import { Form, Grid } from 'semantic-ui-react'

import { useSubstrateState } from './substrate-lib'
import { TxButton } from './substrate-lib/components'

import CourseCards from './CourseCards'

const parseCourse = ({ dna, price, course_year, owner }) => ({
  dna,
  price: price.toJSON(),
  course_year: course_year.toJSON(),
  owner: owner.toJSON(),
})

export default function Courses(props) {
  const { api, keyring } = useSubstrateState()
  const [courseIds, setCourseIds] = useState([])
  const [courses, setCourses] = useState([])
  const [status, setStatus] = useState('')

  const subscribeCount = () => {
    let unsub = null

    const asyncFetch = async () => {
      unsub = await api.query.courseGrading.countForCourses(async count => {
        // Fetch all course keys
        const entries = await api.query.courseGrading.courses.entries()
        const ids = entries.map(entry => entry[1].unwrap().dna)
        setCourseIds(ids)
      })
    }

    asyncFetch()

    return () => {
      unsub && unsub()
    }
  }

  const subscribeCourses = () => {
    let unsub = null

    const asyncFetch = async () => {
      unsub = await api.query.courseGrading.courses.multi(
        courseIds,
        courses => {
          const coursesMap = courses.map(course => parseCourse(course.unwrap()))
          setCourses(coursesMap)
        }
      )
    }

    asyncFetch()

    return () => {
      unsub && unsub()
    }
  }

  useEffect(subscribeCount, [api, keyring])
  useEffect(subscribeCourses, [api, keyring, courseIds])

  return (
    <Grid.Column width={16}>
      <h1>Courses</h1>
      <CourseCards courses={courses} setStatus={setStatus} />
      <Form style={{ margin: '1em 0' }}>
        <Form.Field style={{ textAlign: 'center' }}>
          <TxButton
            label="Create Course"
            type="SIGNED-TX"
            setStatus={setStatus}
            attrs={{
              palletRpc: 'courseGrading',
              callable: 'createCourse',
              inputParams: [],
              paramFields: [],
            }}
          />
        </Form.Field>
      </Form>
      <div style={{ overflowWrap: 'break-word' }}>{status}</div>
    </Grid.Column>
  )
}
