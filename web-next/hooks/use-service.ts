import { usePromise } from '@/hooks/use-promise';
import UserService from '@/grpc-services/user';

const userService  = new UserService();

export default function useService() {
  return {
    user: {
      getMe: usePromise(userService.getMe.bind(userService), [])
    }
  }
}